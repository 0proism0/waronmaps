#!/bin/bash
# Run from your Mac. Copies the live game state (and optionally the node cache)
# to the Tencent Cloud server.
#
# Usage:
#   ./deploy/tencent/migrate.sh ubuntu@YOUR_SERVER_IP

set -euo pipefail

SERVER="${1:-}"
if [[ -z "$SERVER" ]]; then
  echo "Usage: ./deploy/tencent/migrate.sh ubuntu@YOUR_SERVER_IP"
  exit 1
fi

echo "==> Copying game state..."
scp game_data/state.json "${SERVER}:/home/ubuntu/waronmaps/game_data/"

echo ""
echo "The game state has been copied."
echo "If you also want to copy the pre-generated node cache (~1.9 GB), run:"
echo ""
echo "  rsync -avz --progress local_node_store/northern_new_england/ \\"
echo "    ${SERVER}:/home/ubuntu/waronmaps/local_node_store/northern_new_england/"
echo ""
echo "If you skip the cache, the server will download it from OpenStreetMap on first start."
