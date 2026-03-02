#!/usr/bin/env bash
set -euo pipefail

log() { echo "[uninstall] $*"; }
ok()  { echo "[uninstall] ✓ $*"; }

export KUBECONFIG="${HOME}/.kube/config"

log "Deleting backend namespace (all resources inside will be removed)..."
kubectl delete namespace backend --ignore-not-found=true
ok "Namespace 'backend' deleted"

# Optionally remove local Docker images
if [ "${REMOVE_IMAGES:-false}" = "true" ]; then
    log "Removing local Docker images..."
    for img in db-service cache-service product-backend product-frontend; do
        docker rmi "localhost:5000/${img}:latest" --force 2>/dev/null || true
    done
    ok "Docker images removed"
fi

echo ""
echo "═══════════════════════════════════════════════════"
echo " Uninstall complete!"
echo " To also remove Docker images, run:"
echo "   REMOVE_IMAGES=true bash scripts/uninstall.sh"
echo "═══════════════════════════════════════════════════"
