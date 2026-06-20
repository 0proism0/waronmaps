# waronmap

Persistent multiplayer road-based node strategy game built with Rust and MapLibre.

## Rendering And UI

- HTML is only used for 2D overlay UI:
  - login / top status panel
  - node info panel
  - small control buttons
- Nodes are rendered with MapLibre GL, which uses WebGL.
- Connection roads and transport arrows are rendered on a canvas overlay, not as HTML elements.
- The goal is to keep gameplay rendering out of the DOM.

## Networking

- Backend is written in Rust in `rust_server/`.
- Data exchange uses JSON.
- Current live game state and actions are still served through HTTP JSON endpoints.
- Recommended next networking step is a Rust WebSocket JSON channel for world updates, while keeping request-style endpoints only for auth/bootstrap if needed.

## Current Gameplay Flow

1. Register or log in.
2. Click one of your own green nodes.
3. Use the bottom-right node info panel.
4. In `Adjacent Targets`, click `Connect` on a directly adjacent node.
5. Adjust `army per tick` from the same panel.

## Project Structure

- `openfreemap_viewer.html`: main frontend shell, overlays, canvas connection rendering
- `rust_server/src/main.rs`: Rust backend, auth, world state, connection rules, JSON APIs
- `local_node_store/`: local region data and state boundaries
- `vendor/`: local frontend dependencies
- `sw.js`: service worker cache versioning

## Development

Run the app with:

```bash
./resume_node_map.sh
```

Open:

```text
http://localhost:8002/openfreemap_viewer.html
```

## Gitignore

Yes, this project needs a `.gitignore`.

The repository should ignore:

- runtime logs and PID files
- Rust build output
- local generated map/tile caches
- large source PBF files
- private/local game state such as player sessions and passwords
- Python bytecode caches

That is why the root `.gitignore` excludes `.runtime/`, `rust_server/target/`, generated local node cache data, `state_pbf/`, `game_data/state.json`, and `__pycache__/`.

## License

MIT
