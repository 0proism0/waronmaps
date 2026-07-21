# Deploy to Tencent Cloud

Yes, you can host **waronmaps.com** on Tencent Cloud. The easiest path is a Linux cloud server (CVM or Lighthouse) running Nginx + certbot in front of the Rust game server.

## Important: ICP filing (备案)

- **Mainland China regions** (Beijing, Shanghai, Guangzhou, etc.): If you serve a website on ports **80/443**, Tencent Cloud requires the domain to have an **ICP filing** (备案) before they allow public HTTP/HTTPS traffic.
- **Hong Kong / Singapore / Tokyo / Silicon Valley regions**: No ICP filing required. Pick one of these if you want to go live immediately.

If you already have a filed domain, a mainland region is fine and will give lower latency for Chinese players.

## Recommended server spec

- **Tencent Cloud Lighthouse** or **CVM**
- **Region**: Hong Kong or Singapore if no ICP filing; otherwise a mainland region of your choice.
- **OS**: Ubuntu 22.04 LTS
- **CPU / RAM**: 2 vCPU, 4 GB RAM minimum
- **Disk**: 50 GB SSD minimum (the node cache can grow)
- **Bandwidth**: 3–5 Mbps or higher (depends on player count; static assets are small)

## Network setup

1. In the Tencent Cloud console, open the **security group / firewall** for the instance and allow:
   - `TCP 22` (SSH)
   - `TCP 80` (HTTP)
   - `TCP 443` (HTTPS)
2. You do **not** need to open `8002` or `8003` to the internet; Nginx talks to them on `localhost`.
3. Point your domain’s **A record** to the server’s public IP.

## Deployment overview

1. Copy this repository to the server (e.g. `git clone` or `rsync`).
2. Copy your existing `game_data/state.json` to the server to keep player progress.
3. Run `deploy/tencent/setup.sh` on the server to install dependencies, build the Rust server, configure Nginx, and obtain an SSL certificate.
4. Optionally upload `local_node_store/northern_new_england/` to skip regenerating the map cache from OpenStreetMap.

## Quick commands

On your Mac, from the project root:

```bash
# 1. Copy state to the cloud server (replace with your user/IP)
scp game_data/state.json ubuntu@YOUR_SERVER_IP:/home/ubuntu/waronmaps/game_data/

# 2. (Optional) Upload the pre-generated node cache so you don't have to wait for OSM
rsync -avz --progress local_node_store/northern_new_england/ \
  ubuntu@YOUR_SERVER_IP:/home/ubuntu/waronmaps/local_node_store/northern_new_england/
```

Then SSH into the server and run:

```bash
cd /home/ubuntu/waronmap
sudo ./deploy/tencent/setup.sh your-email@example.com
```

The script will:
- Install Rust, Nginx, and certbot
- Build the game server
- Start it as a systemd service
- Configure Nginx reverse proxy
- Request a Let’s Encrypt certificate for `waronmaps.com`

After it finishes, the game is available at:

```
https://waronmaps.com/openfreemap_viewer.html
```

## Letting players connect

Players just open `https://waronmaps.com/openfreemap_viewer.html` in a browser. Nginx terminates HTTPS and forwards WebSocket traffic on `/ws` to the Rust server.

## Maintenance

```bash
# View game server logs
sudo journalctl -u waronmaps -f

# Restart the game server
sudo systemctl restart waronmaps

# Reload Nginx after config changes
sudo systemctl reload nginx

# Test SSL renewal
sudo certbot renew --dry-run
```
