#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(dirname "$SCRIPT_DIR")"
REGISTRY="localhost:5000"

log() { echo "[build-deploy] $*"; }
ok()  { echo "[build-deploy] ✓ $*"; }

# ─── Check prerequisites ──────────────────────────────────────────────────────
for cmd in docker kubectl cargo npm; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "ERROR: $cmd not found. Run scripts/setup-k8s.sh first."
        exit 1
    fi
done

if ! docker info &>/dev/null 2>&1; then
    echo "ERROR: Cannot connect to Docker daemon (permission denied on docker socket)."
    echo "       You were recently added to the 'docker' group. Activate it with:"
    echo "         newgrp docker"
    echo "       Then re-run this script."
    exit 1
fi

if [ ! -f "${HOME}/.kube/config" ]; then
    echo "ERROR: ~/.kube/config not found. Run scripts/setup-k8s.sh first."
    exit 1
fi

export KUBECONFIG="${HOME}/.kube/config"

# ─── Build Rust workspace ─────────────────────────────────────────────────────
log "Building Rust workspace..."
cd "$BACKEND_DIR"
cargo build --release
ok "Rust build complete"

# ─── Build frontend ───────────────────────────────────────────────────────────
log "Building frontend..."
cd "$BACKEND_DIR/product-frontend"
npm ci
npm run build
ok "Frontend build complete"
cd "$BACKEND_DIR"

# ─── Build Docker images ──────────────────────────────────────────────────────
log "Building Docker images..."

docker build \
    -f docker/db-service.Dockerfile \
    -t "${REGISTRY}/db-service:latest" \
    .
ok "db-service image built"

docker build \
    -f docker/cache-service.Dockerfile \
    -t "${REGISTRY}/cache-service:latest" \
    .
ok "cache-service image built"

docker build \
    -f docker/product-backend.Dockerfile \
    -t "${REGISTRY}/product-backend:latest" \
    .
ok "product-backend image built"

# Get VITE_GOOGLE_CLIENT_ID from the secret (if it's been configured)
GOOGLE_CLIENT_ID=""
if kubectl get secret experiproduct-secret -n backend &>/dev/null 2>&1; then
    GOOGLE_CLIENT_ID=$(kubectl get secret experiproduct-secret -n backend \
        -o jsonpath='{.data.GOOGLE_CLIENT_ID}' | base64 -d 2>/dev/null || echo "")
fi

# Read GOOGLE_AUTH_ENABLED from the deployment manifest so the frontend
# build flag always matches what the backend will receive at runtime.
GOOGLE_AUTH_ENABLED=$(grep "GOOGLE_AUTH_ENABLED" "$BACKEND_DIR/k8s/experiproduct/deployment.yaml" \
    | awk '{print $2}' | tr -d '"' | tail -1)
GOOGLE_AUTH_ENABLED="${GOOGLE_AUTH_ENABLED:-false}"

docker build \
    -f docker/product-frontend.Dockerfile \
    --build-arg "VITE_GOOGLE_CLIENT_ID=${GOOGLE_CLIENT_ID}" \
    --build-arg "VITE_GOOGLE_AUTH_ENABLED=${GOOGLE_AUTH_ENABLED}" \
    -t "${REGISTRY}/product-frontend:latest" \
    .
ok "product-frontend image built"

# ─── Push to local registry ───────────────────────────────────────────────────
log "Pushing images to local registry..."
docker push "${REGISTRY}/db-service:latest"
docker push "${REGISTRY}/cache-service:latest"
docker push "${REGISTRY}/product-backend:latest"
docker push "${REGISTRY}/product-frontend:latest"
ok "All images pushed"

# ─── Apply K8s manifests ─────────────────────────────────────────────────────
log "Applying K8s manifests..."

# Namespace first
kubectl apply -f "$BACKEND_DIR/k8s/namespace.yaml"

# Apply all manifests (all are idempotent)
kubectl apply -f "$BACKEND_DIR/k8s/registry/"
kubectl apply -f "$BACKEND_DIR/k8s/postgresql/"
kubectl apply -f "$BACKEND_DIR/k8s/redis/"
kubectl apply -f "$BACKEND_DIR/k8s/loki/"
kubectl apply -f "$BACKEND_DIR/k8s/grafana/"
kubectl apply -f "$BACKEND_DIR/k8s/experiproduct/"

ok "All manifests applied"

# ─── Restart deployments to pick up new images ───────────────────────────────
log "Restarting deployments..."
kubectl rollout restart deployment/postgresql -n backend
kubectl rollout restart deployment/redis -n backend
kubectl rollout restart deployment/loki -n backend
kubectl rollout restart deployment/grafana -n backend
kubectl rollout restart deployment/experiproduct -n backend

# ─── Wait for rollouts ────────────────────────────────────────────────────────
log "Waiting for deployments to be ready..."
kubectl rollout status deployment/postgresql -n backend --timeout=120s
kubectl rollout status deployment/redis -n backend --timeout=60s
kubectl rollout status deployment/loki -n backend --timeout=120s
kubectl rollout status deployment/grafana -n backend --timeout=60s
kubectl rollout status deployment/experiproduct -n backend --timeout=120s

ok "All deployments ready"

# ─── Print access URLs ────────────────────────────────────────────────────────
NODE_IP=$(kubectl get nodes -o jsonpath='{.items[0].status.addresses[?(@.type=="InternalIP")].address}')

echo ""
echo "═══════════════════════════════════════════════════"
echo " Deployment complete!"
echo ""
echo " Access URLs:"
echo "   App:     http://${NODE_IP}"
echo "   Grafana: http://${NODE_IP}:30300"
echo ""
echo " Pod status:"
kubectl get pods -n backend
echo "═══════════════════════════════════════════════════"
