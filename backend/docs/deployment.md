# Deployment

This page covers the full lifecycle: first-time infrastructure setup, building and deploying the application, and tearing it down.

## Prerequisites

- Ubuntu machine (22.04 or 24.04 recommended)
- `sudo` access
- Internet connectivity (for downloading Docker, k3s, Rust toolchain)
- A Google Cloud project with an OAuth 2.0 Client ID

---

## Step 1 — Set Up Infrastructure

Run once on a fresh machine. The script is idempotent — safe to re-run.

```bash
bash backend/scripts/setup-k8s.sh
```

What it does:
1. Installs **Docker CE** from the official apt repository
2. Installs **k3s** with `--docker` (uses Docker as the container runtime instead of containerd)
3. Waits for k3s to be ready, then copies the kubeconfig to `~/.kube/config`
4. Writes `/etc/rancher/k3s/registries.yaml` to allow k3s to pull from `localhost:5000` over plain HTTP
5. Restarts k3s and waits for it to come back up
6. Installs `protoc` (Protocol Buffer compiler, needed for Rust gRPC codegen)
7. Applies the local registry deployment to k3s
8. Starts a `registry:2` Docker container on the host at port 5000

After this script completes, verify:

```bash
kubectl get nodes             # Should show STATUS=Ready
docker ps | grep registry     # Should show local-registry on port 5000
```

> If `kubectl` isn't in your PATH after installation, run:
> ```bash
> export KUBECONFIG=~/.kube/config
> ```
> Add this to your shell profile for persistence.

---

## Step 2 — Configure Secrets

Before deploying, set real values in the experiproduct secret file:

```bash
$EDITOR backend/k8s/experiproduct/secret.yaml
```

Set:
- **`GOOGLE_CLIENT_ID`** — from [Google Cloud Console](https://console.cloud.google.com) → APIs & Services → Credentials → OAuth 2.0 Client IDs
- **`GOOGLE_CLIENT_SECRET`** — same location
- **`JWT_SECRET`** — a strong random string, minimum 32 characters

Generate a secure `JWT_SECRET`:
```bash
openssl rand -hex 32
```

### Google OAuth Setup

1. Go to [Google Cloud Console](https://console.cloud.google.com)
2. Create a project (or select an existing one)
3. Enable the **Google Identity** API
4. Go to **APIs & Services → Credentials → Create Credentials → OAuth client ID**
5. Application type: **Web application**
6. Authorised JavaScript origins: `http://<your-node-ip>:30080`
7. Save the Client ID and Client Secret

> The Client Secret isn't used directly in the current authentication flow (the frontend only sends the `id_token`), but it's stored in the secret for future use.

---

## Step 3 — Build and Deploy

```bash
bash backend/scripts/build-deploy.sh
```

What it does:
1. **Rust build** — `cargo build --release` for all three services (uses Cargo's incremental cache)
2. **Frontend build** — `npm ci && npm run build` in `product-frontend/`
3. **Docker builds** — 4 multi-stage images tagged as `localhost:5000/<name>:latest`
   - The `VITE_GOOGLE_CLIENT_ID` build arg is read from the k3s secret automatically
4. **Docker push** — all 4 images pushed to `localhost:5000`
5. **kubectl apply** — applies all manifests in dependency order (namespace → datastores → observability → app)
6. **Rollout restart** — forces all deployments to pull the latest image
7. **Rollout status** — waits for each deployment to reach the Ready state
8. **Prints access URLs** using the node's internal IP

### First-Time Deployment Notes

- The first Rust build will take several minutes (compiling from scratch)
- PostgreSQL takes ~10-15 seconds to initialize before `db-service` is ready
- Loki has a 15-second readiness delay while it loads the TSDB index
- Total time from `build-deploy.sh` to all pods Running: typically 3-5 minutes

---

## Step 4 — Verify

```bash
kubectl get pods -n backend
```

Expected output (all pods `2/2` or `1/1 Running`):

```
NAME                            READY   STATUS
postgresql-xxx                  2/2     Running
redis-xxx                       2/2     Running
loki-xxx                        1/1     Running
grafana-xxx                     1/1     Running
experiproduct-xxx               2/2     Running
registry-xxx                    1/1     Running
```

Test the health endpoint:
```bash
NODE_IP=$(kubectl get nodes -o jsonpath='{.items[0].status.addresses[?(@.type=="InternalIP")].address}')
curl http://${NODE_IP}:30080/api/health
# {"status":"ok"}
```

Open in browser:
- **App**: `http://<node-ip>:30080`
- **Grafana**: `http://<node-ip>:30300`

### End-to-End Verification

1. Visit `http://<node-ip>:30080` → Welcome page with Google sign-in
2. Sign in with Google → redirected to Items page
3. Create an item → appears in the table
4. Edit and delete items → changes reflected immediately
5. Visit `/redis` → see heartbeat keys appearing every 5 seconds
6. Visit `/telemetry` → Grafana loads in iframe
7. In Grafana Explore, query `{service="product-backend"}` → see auth and CRUD events
8. Visit `/architecture` → PlantUML diagram renders

---

## Redeploying After Code Changes

Run `build-deploy.sh` again — it rebuilds only what changed (Cargo incremental compilation, Docker layer cache) and rolls out the new images.

```bash
bash backend/scripts/build-deploy.sh
```

To redeploy only the frontend (faster):
```bash
cd backend/product-frontend && npm run build && cd ..
docker build -f docker/product-frontend.Dockerfile -t localhost:5000/product-frontend:latest .
docker push localhost:5000/product-frontend:latest
kubectl rollout restart deployment/experiproduct -n backend
kubectl rollout status deployment/experiproduct -n backend
```

---

## Updating Secrets

To change credentials without a full redeploy:

```bash
# Update the secret
kubectl create secret generic experiproduct-secret -n backend \
  --from-literal=GOOGLE_CLIENT_ID=<new-id> \
  --from-literal=GOOGLE_CLIENT_SECRET=<new-secret> \
  --from-literal=JWT_SECRET=<new-secret> \
  --dry-run=client -o yaml | kubectl apply -f -

# Restart the deployment to pick up the new values
kubectl rollout restart deployment/experiproduct -n backend
```

---

## Tear Down

Delete all application resources (keeps k3s and Docker installed):

```bash
bash backend/scripts/uninstall.sh
```

This runs `kubectl delete namespace backend`, which cascades to all Deployments, Services, PVCs, Secrets, and ConfigMaps.

To also remove the locally built Docker images:
```bash
REMOVE_IMAGES=true bash backend/scripts/uninstall.sh
```

To redeploy from scratch after uninstalling:
```bash
bash backend/scripts/build-deploy.sh
```

---

## Uninstalling k3s Completely

If you want to remove k3s from the machine entirely:

```bash
/usr/local/bin/k3s-uninstall.sh
```

This script is installed by k3s during setup. It removes k3s, its data directory, and all Kubernetes resources.

---

## Troubleshooting

### Pod stuck in `Pending`

```bash
kubectl describe pod -n backend <pod-name>
```

Common causes:
- **PVC not bound** — check `kubectl get pvc -n backend`; may need to wait for the default StorageClass to provision
- **Image not found** — verify the image was pushed: `curl http://localhost:5000/v2/_catalog`

### Pod stuck in `CrashLoopBackOff`

```bash
kubectl logs -n backend <pod-name> -c <container-name> --previous
```

Common causes:
- **db-service** crashes if PostgreSQL isn't ready yet — the init container should prevent this, but check pg_isready
- **product-backend** fails to connect to db-service or cache-service on first boot — usually resolves within 1-2 restarts as the other pods become ready

### `kubectl` returns "connection refused"

k3s may not be running:
```bash
sudo systemctl status k3s
sudo systemctl start k3s
```

### Frontend shows a blank page after deploy

Nginx may be serving a stale cached version. Hard-refresh with `Ctrl+Shift+R`. If the issue persists:
```bash
kubectl rollout restart deployment/experiproduct -n backend
```

### Google sign-in fails with "Token audience mismatch"

The `GOOGLE_CLIENT_ID` in the k3s secret doesn't match what the frontend bundle was built with. Rebuild the frontend image after updating the secret:
```bash
bash backend/scripts/build-deploy.sh
```

### Grafana iframe shows a blank page

Check that `GF_SECURITY_ALLOW_EMBEDDING=true` is set and Grafana is running:
```bash
kubectl logs -n backend deployment/grafana
curl http://localhost:30300/api/health
```
