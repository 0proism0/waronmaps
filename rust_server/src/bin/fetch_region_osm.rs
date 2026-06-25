// Fetches OpenStreetMap road data for the active region from the Overpass API
// and derives the prepared node dataset the runtime expects.
//
// Output (under the region's prepared_root):
//   _overpass_cache/osm_<query_id>.json  raw Overpass responses (nodes + ways),
//                                        read directly by the runtime for adjacency
//   intersections.csv                    the playable game nodes (road junctions)
//
// After this runs, `generate_nodes` re-sorts intersections.csv by the S2/Hilbert
// curve and writes the preview/index files, then `prepare_region_cache` refreshes
// the cached status files. This binary replaces the data-fetching role that the
// removed Python builder used to perform.
//
// Network access is delegated to the system `curl` so the runtime keeps its small
// dependency set (no HTTP/TLS stack pulled into the server build).

use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const OVERPASS_URL: &str = "https://overpass-api.de/api/interpreter";
// Road classes pulled into the playable graph. Tighten or widen this to trade off
// intersection density against download size. `residential` is intentionally left
// out: in rural Maine/New Hampshire it explodes the node count without adding much
// strategic value.
const HIGHWAY_FILTER: &str = "motorway|trunk|primary|secondary|tertiary|unclassified|motorway_link|trunk_link|primary_link|secondary_link|tertiary_link";
const OVERPASS_TIMEOUT_SECS: u64 = 180;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(env::current_dir().map_err(|err| err.to_string())?);

    let manifest_path = root.join("local_node_store").join("region_manifest.json");
    let manifest = read_json_value(&manifest_path)?;
    let region = manifest
        .get("active_region")
        .cloned()
        .ok_or_else(|| "Missing active_region in region manifest.".to_string())?;
    let prepared_root = root.join(
        region
            .get("prepared_root")
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing prepared_root in region manifest.".to_string())?,
    );
    let cache_dir = prepared_root.join("_overpass_cache");
    fs::create_dir_all(&cache_dir).map_err(|err| err.to_string())?;

    let query_bounds = query_bound_boxes(&region)?;
    println!(
        "Fetching OSM highways for {} query area(s) into {}...",
        query_bounds.len(),
        cache_dir.display()
    );

    for (id, bbox) in &query_bounds {
        let cache_path = cache_dir.join(format!("osm_{id}.json"));
        if file_is_nonempty(&cache_path) {
            println!("  [{id}] cache present, skipping download.");
            continue;
        }
        println!(
            "  [{id}] querying Overpass (S {:.3}, W {:.3}, N {:.3}, E {:.3})...",
            bbox.south, bbox.west, bbox.north, bbox.east
        );
        let body = fetch_overpass_with_retries(bbox, id)?;
        fs::write(&cache_path, &body).map_err(|err| err.to_string())?;
        println!("  [{id}] saved {} bytes.", body.len());
    }

    let intersections_csv_path = prepared_root.join("intersections.csv");
    build_intersections_csv(&cache_dir, &intersections_csv_path)?;
    Ok(())
}

struct BBox {
    west: f64,
    east: f64,
    south: f64,
    north: f64,
}

fn query_bound_boxes(region: &Value) -> Result<Vec<(String, BBox)>, String> {
    if let Some(bounds_list) = region.get("query_bounds").and_then(Value::as_array) {
        let mut boxes = Vec::new();
        for (index, bounds) in bounds_list.iter().enumerate() {
            let id = bounds
                .get("id")
                .and_then(Value::as_str)
                .map(|value| value.to_string())
                .unwrap_or_else(|| format!("tile_{index}"));
            boxes.push((id, bbox_from_value(bounds)?));
        }
        return Ok(boxes);
    }
    let bounds = region
        .get("bounds")
        .ok_or_else(|| "Region has neither query_bounds nor bounds.".to_string())?;
    Ok(vec![("region".to_string(), bbox_from_value(bounds)?)])
}

fn bbox_from_value(value: &Value) -> Result<BBox, String> {
    let read = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("Missing numeric '{key}' in query bounds."))
    };
    Ok(BBox {
        west: read("west")?,
        east: read("east")?,
        south: read("south")?,
        north: read("north")?,
    })
}

const MAX_ATTEMPTS: u32 = 4;

fn fetch_overpass_with_retries(bbox: &BBox, id: &str) -> Result<Vec<u8>, String> {
    // Overpass's dispatcher transiently rejects requests (429/504, and sometimes a
    // 406 from the load balancer) under load, so retry with linear backoff.
    let mut last_error = String::new();
    for attempt in 1..=MAX_ATTEMPTS {
        match fetch_overpass(bbox).and_then(|body| validate_overpass_json(body, id)) {
            Ok(body) => return Ok(body),
            Err(err) => {
                last_error = err;
                if attempt < MAX_ATTEMPTS {
                    let backoff = 10 * attempt as u64;
                    eprintln!("  [{id}] attempt {attempt} failed: {last_error}; retrying in {backoff}s...");
                    std::thread::sleep(std::time::Duration::from_secs(backoff));
                }
            }
        }
    }
    Err(format!("[{id}] giving up after {MAX_ATTEMPTS} attempts: {last_error}"))
}

fn fetch_overpass(bbox: &BBox) -> Result<Vec<u8>, String> {
    // Overpass bbox order is (south, west, north, east).
    let query = format!(
        "[out:json][timeout:{timeout}];way[\"highway\"~\"^({filter})$\"]({south},{west},{north},{east});(._;>;);out;",
        timeout = OVERPASS_TIMEOUT_SECS,
        filter = HIGHWAY_FILTER,
        south = bbox.south,
        west = bbox.west,
        north = bbox.north,
        east = bbox.east,
    );
    let output = Command::new("curl")
        .arg("-sS")
        .arg("--max-time")
        .arg((OVERPASS_TIMEOUT_SECS + 60).to_string())
        .arg("-A")
        .arg("waronmap-fetch-region-osm/1.0")
        .arg("-G")
        .arg(OVERPASS_URL)
        .arg("--data-urlencode")
        .arg(format!("data={query}"))
        .output()
        .map_err(|err| format!("Failed to launch curl: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("curl failed: {stderr}"));
    }
    Ok(output.stdout)
}

fn validate_overpass_json(body: Vec<u8>, id: &str) -> Result<Vec<u8>, String> {
    let value: Value = serde_json::from_slice(&body).map_err(|_| {
        let preview = String::from_utf8_lossy(&body);
        let preview = preview.trim();
        let preview: String = preview.chars().take(160).collect();
        format!("[{id}] Overpass returned non-JSON (likely rate-limit/error page): {preview}")
    })?;
    let count = value
        .get("elements")
        .and_then(Value::as_array)
        .map(|items| items.len())
        .unwrap_or(0);
    if count == 0 {
        return Err(format!(
            "[{id}] Overpass returned 0 elements; refusing to cache an empty tile."
        ));
    }
    Ok(body)
}

fn build_intersections_csv(cache_dir: &Path, csv_path: &Path) -> Result<(), String> {
    let mut coords: HashMap<String, (f64, f64)> = HashMap::new();
    // For each node id: how many distinct ways reference it, and its segment degree
    // (count of incident consecutive-pair segments across all ways).
    let mut way_count: HashMap<String, u32> = HashMap::new();
    let mut seg_degree: HashMap<String, u32> = HashMap::new();

    let mut cache_files = fs::read_dir(cache_dir)
        .map_err(|err| err.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    cache_files.sort();

    for path in &cache_files {
        let value = read_json_value(path)?;
        let elements = value
            .get("elements")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for element in &elements {
            match element.get("type").and_then(Value::as_str) {
                Some("node") => {
                    if let (Some(id), Some(lat), Some(lon)) = (
                        node_id_string(element.get("id")),
                        element.get("lat").and_then(Value::as_f64),
                        element.get("lon").and_then(Value::as_f64),
                    ) {
                        coords.insert(id, (lat, lon));
                    }
                }
                Some("way") => {
                    let node_ids: Vec<String> = element
                        .get("nodes")
                        .and_then(Value::as_array)
                        .map(|items| items.iter().filter_map(|v| node_id_string(Some(v))).collect())
                        .unwrap_or_default();
                    if node_ids.len() < 2 {
                        continue;
                    }
                    let mut unique = HashSet::new();
                    for id in &node_ids {
                        if unique.insert(id.clone()) {
                            *way_count.entry(id.clone()).or_insert(0) += 1;
                        }
                    }
                    for pair in node_ids.windows(2) {
                        *seg_degree.entry(pair[0].clone()).or_insert(0) += 1;
                        *seg_degree.entry(pair[1].clone()).or_insert(0) += 1;
                    }
                }
                _ => {}
            }
        }
    }

    // A node is a playable intersection if it is a real junction: shared by 2+ ways,
    // a junction within the graph (degree >= 3), or a dead-end endpoint (degree 1).
    // Pure geometry vertices (degree 2, single way) are skipped.
    let mut intersections: BTreeMap<String, (f64, f64, u32)> = BTreeMap::new();
    for (id, (lat, lon)) in &coords {
        let ways = way_count.get(id).copied().unwrap_or(0);
        let degree = seg_degree.get(id).copied().unwrap_or(0);
        if ways >= 2 || degree == 1 || degree >= 3 {
            intersections.insert(id.clone(), (*lat, *lon, degree));
        }
    }

    if intersections.len() < 10 {
        return Err(format!(
            "Only {} intersections found; need at least 10. Check the region bounds or highway filter.",
            intersections.len()
        ));
    }

    let file = fs::File::create(csv_path).map_err(|err| err.to_string())?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "id,latitude,longitude,degree").map_err(|err| err.to_string())?;
    for (id, (lat, lon, degree)) in &intersections {
        writeln!(writer, "{id},{lat:.7},{lon:.7},{degree}").map_err(|err| err.to_string())?;
    }
    writer.flush().map_err(|err| err.to_string())?;
    println!(
        "Wrote {} intersections to {}.",
        intersections.len(),
        csv_path.display()
    );
    Ok(())
}

fn node_id_string(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn file_is_nonempty(path: &Path) -> bool {
    fs::metadata(path).map(|meta| meta.len() > 0).unwrap_or(false)
}

fn read_json_value(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|err| format!("{}: {err}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|err| format!("{}: {err}", path.display()))
}
