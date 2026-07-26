#!/usr/bin/env python3
"""Rebuild node_display_names.json road names from the Overpass cache.

For every way in the Overpass cache, collect its name (or ref) for each node
it passes through. Then rewrite node_display_names.json so that:
  - nodes previously named "(Node <id>)(intersection)" get real road names
  - the multi-road separator becomes lowercase " x "
Output format stays "(Road A x Road B)(intersection)" to match the server.
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CACHE_DIR = ROOT / "local_node_store" / "northern_new_england" / "_overpass_cache"
NAMES_PATH = ROOT / "local_node_store" / "northern_new_england" / "node_display_names.json"


def road_label(tags: dict) -> str:
    name = (tags.get("name") or "").strip()
    if name:
        return name
    ref = (tags.get("ref") or "").strip()
    if ref:
        # refs like "US 1;ME 3" -> keep first segment
        return ref.split(";")[0].strip()
    return ""


def main() -> None:
    node_names: dict[str, set[str]] = {}
    files = sorted(CACHE_DIR.glob("*.json"))
    print(f"Scanning {len(files)} cache files...")
    for i, path in enumerate(files, 1):
        try:
            with path.open() as handle:
                data = json.load(handle)
        except Exception as exc:
            print(f"  skip {path.name}: {exc}")
            continue
        for element in data.get("elements", []):
            if element.get("type") != "way":
                continue
            label = road_label(element.get("tags", {}))
            if not label:
                continue
            for node_ref in element.get("nodes", []):
                key = str(node_ref)
                node_names.setdefault(key, set()).add(label)
        if i % 100 == 0:
            print(f"  {i}/{len(files)} files, {len(node_names)} nodes with names")

    with NAMES_PATH.open() as handle:
        names = json.load(handle)

    fixed_fallback = 0
    lowered = 0
    for node_id, raw in names.items():
        new_raw = raw
        if raw.startswith("(Node "):
            labels = node_names.get(node_id)
            if labels:
                ordered = sorted(labels)
                new_raw = f"({' x '.join(ordered)})(intersection)"
                fixed_fallback += 1
        if " X " in new_raw:
            new_raw = new_raw.replace(" X ", " x ")
            lowered += 1
        names[node_id] = new_raw

    with NAMES_PATH.open("w") as handle:
        json.dump(names, handle)

    remaining = sum(1 for v in names.values() if v.startswith("(Node "))
    print(f"Fixed fallback nodes: {fixed_fallback}")
    print(f"Lowercased separators: {lowered}")
    print(f"Remaining '(Node ...)' entries: {remaining}")


if __name__ == "__main__":
    sys.exit(main())
