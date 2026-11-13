#!/usr/bin/env bash
set -e

echo "=== 1. Updating System & Installing Build Dependencies ==="
sudo apt-get update -y
sudo apt-get install -y build-essential pkg-config libssl-dev curl git

echo "=== 2. Opening Oracle VM Linux Firewall for Port 8080, 80, 443 ==="
# Oracle Linux/Ubuntu images have default iptables rules that block incoming ports
sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 8080 -j ACCEPT
sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 80 -j ACCEPT
sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 443 -j ACCEPT
sudo netfilter-persistent save 2>/dev/null || sudo iptables-save | sudo tee /etc/iptables/rules.v4

echo "=== 3. Installing Rust Toolchain ==="
if ! command -v rustc &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

echo "=== 4. Setting up .env configuration ==="
if [ ! -f .env ]; then
    cp .env.example .env
    echo ".env created from .env.example"
fi

echo "=== 5. Compiling Optimized Release Binary ==="
source "$HOME/.cargo/env"
cargo build --release

echo "=== 6. Installing & Enabling Systemd Background Service ==="
sudo cp deploy/geoguide.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable geoguide
sudo systemctl restart geoguide

echo ""
echo "=== GeoGuide is now running 24/7 on your Oracle VM! ==="
echo "Check service status: sudo systemctl status geoguide"
echo "View live logs:       sudo journalctl -u geoguide -f"
echo "Test endpoint:        curl http://localhost:8080/health"
