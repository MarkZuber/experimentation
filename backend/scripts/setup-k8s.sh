#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(dirname "$SCRIPT_DIR")"

log() { echo "[setup-k8s] $*"; }
ok()  { echo "[setup-k8s] ✓ $*"; }

# ─── Build dependencies (libssl-dev, protoc, etc.) ────────────────────────────
log "Installing build dependencies..."
bash "$SCRIPT_DIR/install-build-deps.sh"

# ─── Docker (for image builds — k3s uses its own containerd runtime) ──────────
if command -v docker &>/dev/null; then
    ok "Docker already installed ($(docker --version))"
else
    log "Installing Docker CE..."
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | \
        sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
    echo \
        "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] \
        https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | \
        sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
    sudo apt-get update -q
    sudo apt-get install -y docker-ce docker-ce-cli containerd.io
    sudo usermod -aG docker "$USER"
    ok "Docker CE installed"
    echo ""
    echo "  ⚠  You have been added to the 'docker' group."
    echo "     Log out and back in (or run 'newgrp docker') before running build-deploy.sh."
    echo ""
fi

# ─── k3s ───────────────────────────────────────────────────────────────────────
# containerd runtime (default — do NOT use --docker, it breaks cluster DNS).
# --disable=traefik   we expose port 80 via our own LoadBalancer service.
# --write-kubeconfig-mode=644  makes the kubeconfig readable without sudo.
if command -v k3s &>/dev/null; then
    ok "k3s already installed ($(k3s --version | head -1))"
else
    log "Installing k3s..."
    curl -sfL https://get.k3s.io | sh -s -- \
        --disable=traefik \
        --write-kubeconfig-mode=644
    ok "k3s installed"
fi

# ─── Wait for k3s to be ready ─────────────────────────────────────────────────
log "Waiting for k3s to be ready..."
for i in $(seq 1 30); do
    if k3s kubectl get nodes &>/dev/null 2>&1; then
        ok "k3s is ready"
        break
    fi
    echo "  Attempt $i/30 — waiting 5s..."
    sleep 5
done

# ─── kubeconfig ───────────────────────────────────────────────────────────────
mkdir -p "$HOME/.kube"
if [ -r /etc/rancher/k3s/k3s.yaml ]; then
    cp /etc/rancher/k3s/k3s.yaml "$HOME/.kube/config"
else
    sudo cp /etc/rancher/k3s/k3s.yaml "$HOME/.kube/config"
    sudo chown "$USER":"$USER" "$HOME/.kube/config"
fi
export KUBECONFIG="$HOME/.kube/config"
ok "kubeconfig configured at ~/.kube/config"

# ─── Local registry insecure config ───────────────────────────────────────────
REGISTRIES_YAML="/etc/rancher/k3s/registries.yaml"
if [ ! -f "$REGISTRIES_YAML" ] || ! grep -q "localhost:5000" "$REGISTRIES_YAML" 2>/dev/null; then
    log "Configuring k3s insecure local registry..."
    sudo mkdir -p /etc/rancher/k3s
    sudo tee "$REGISTRIES_YAML" > /dev/null <<'EOF'
mirrors:
  "localhost:5000":
    endpoint:
      - "http://localhost:5000"
EOF
    sudo systemctl restart k3s
    log "Waiting for k3s to restart..."
    sleep 10
    for i in $(seq 1 30); do
        if kubectl get nodes &>/dev/null 2>&1; then
            ok "k3s restarted"
            break
        fi
        sleep 3
    done
    ok "Local registry configured in k3s"
else
    ok "k3s registry config already present"
fi

# ─── Local Docker registry on host port 5000 ─────────────────────────────────
if sudo docker ps --format '{{.Names}}' 2>/dev/null | grep -q "^local-registry$"; then
    ok "Host registry container already running on port 5000"
else
    log "Starting local Docker registry container on port 5000..."
    sudo docker run -d --restart=always --name local-registry -p 5000:5000 registry:2
    ok "Local registry container started on port 5000"
fi

# ─── Deploy namespace + registry into k3s ────────────────────────────────────
log "Deploying registry to k3s..."
kubectl apply -f "$BACKEND_DIR/k8s/namespace.yaml"
kubectl apply -f "$BACKEND_DIR/k8s/registry/"
log "Waiting for registry pod..."
kubectl rollout status deployment/registry -n backend --timeout=60s
ok "Local registry running"

echo ""
echo "═══════════════════════════════════════════════════"
echo " setup-k8s.sh complete!"
echo ""
echo " Next steps:"
echo "   1. Configure Google OAuth:"
echo "      Edit backend/k8s/experiproduct/secret.yaml"
echo "   2. Build and deploy:"
echo "      bash backend/scripts/build-deploy.sh"
echo "═══════════════════════════════════════════════════"
