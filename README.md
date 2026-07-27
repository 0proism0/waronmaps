# Fight and Conquer

A real-time multiplayer strategy game played on the actual road map. Players claim
road intersections, raise armies on them, link them together, and fight over
territory that never resets. The whole backend is a single Rust server; the map is
rendered in the browser with MapLibre.

## How it plays

You register (or play as a guest), pick a neutral intersection as your starting
point, and get a small cluster of nodes to call your own. Nodes slowly build army.
Drag from one of your nodes to another to open a link that moves troops every tick —
reinforce your own nodes or attack someone else's. Adjacent links move at whatever
rate you set; long "irregular" links that skip across the map crawl along at a
fraction of that. There's a chat, a leaderboard, and a phone-friendly connect mode
so you don't need a keyboard.

## What's in here

- `openfreemap_viewer.html` — the entire frontend (map, panels, canvas overlays).
- `rust_server/src/main.rs` — the game server: auth, world state, combat ticks,
  WebSocket broadcasts, chat, and the JSON API.
- `rust_server/src/bin/` — small tools that pull OSM road data from Overpass and
  turn it into the playable node set (`fetch_region_osm`, `generate_nodes`,
  `prepare_region_cache`).
- `local_node_store/` — prepared region data (gitignored, regenerated on demand).
- `vendor/`, `sw.js` — frontend deps and cache versioning.

## Running it

Build and start the server from the repo root:

```bash
cargo build --release --manifest-path rust_server/Cargo.toml
./rust_server/target/release/node_game_server
```

Then open `http://localhost:8002/openfreemap_viewer.html`. The HTTP API is on port
8002, the WebSocket stream on 8003.

If the prepared node data is missing, the first run fetches OSM roads from Overpass
(one-time, a few minutes) and builds the S2/Hilbert-sorted node index. To force a
rebuild, delete `_overpass_cache/` and `intersections.csv` inside the prepared
region folder, then run the fetch and generate tools again.

Game state lives in `game_data/state.json` by default. Point `DATABASE_URL` at a
Postgres database (e.g. Neon) and it'll persist there instead, keeping the local
file as a backup.

## License

MIT
