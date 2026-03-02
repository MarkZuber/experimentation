# Kubernetes

All resources live in a single `backend` namespace on a k3s cluster.

## Namespace

```yaml
# k8s/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: backend
```

Apply first — all other manifests depend on it.

## Cluster Overview

```
kubectl get all -n backend

NAME                                READY   STATUS    RESTARTS
pod/postgresql-<hash>               2/2     Running   0       ← postgres + db-service
pod/redis-<hash>                    2/2     Running   0       ← redis + cache-service
pod/loki-<hash>                     1/1     Running   0
pod/grafana-<hash>                  1/1     Running   0
pod/experiproduct-<hash>            2/2     Running   0       ← product-backend + nginx

NAME                 TYPE        CLUSTER-IP    PORT(S)
svc/db-service       ClusterIP   10.x.x.x      5432/TCP,50051/TCP
svc/cache-service    ClusterIP   10.x.x.x      6379/TCP,50052/TCP
svc/loki             ClusterIP   10.x.x.x      3100/TCP
svc/grafana          NodePort    10.x.x.x      3000:30300/TCP
svc/experiproduct    NodePort    10.x.x.x      80:30080/TCP
svc/registry         NodePort    10.x.x.x      5000:30500/TCP
```

---

## postgresql

**Manifests**: `k8s/postgresql/`

### Secret (`secret.yaml`)

Contains all PostgreSQL credentials. The `DATABASE_URL` bundles them into a single connection string for `db-service`.

| Key | Value |
|-----|-------|
| `POSTGRES_USER` | `experiproduct` |
| `POSTGRES_PASSWORD` | `experiproduct-pg-pass` |
| `POSTGRES_DB` | `experiproduct` |
| `DATABASE_URL` | `postgres://experiproduct:experiproduct-pg-pass@localhost:5432/experiproduct` |

> These are defaults. For production use, override with `kubectl create secret --from-literal`.

### PVC (`pvc.yaml`)

```yaml
name: postgresql-pvc
size: 5Gi
accessMode: ReadWriteOnce
```

Mounted at `/var/lib/postgresql/data` in the postgres container. `PGDATA` is set to the `pgdata` subdirectory to avoid permission issues with the volume root.

### Deployment (`deployment.yaml`)

Two containers in one pod:

**Init container** — `wait-for-postgres`: polls `pg_isready` before the `db-service` container starts. This prevents db-service from crashing on first boot while PostgreSQL is still initialising.

**postgres container** (image: `postgres:15`):
- Gets credentials from the Secret
- Readiness probe: `pg_isready -U experiproduct` (every 5s, initial delay 5s)
- Mounts PVC at `/var/lib/postgresql/data`

**db-service container** (image: `localhost:5000/db-service:latest`):
- Gets `DATABASE_URL` from the Secret
- Gets `LOKI_URL` from env literal
- No readiness probe (gRPC health not configured; rely on pod-level readiness)

### Service (`service.yaml`)

ClusterIP service named `db-service` — this is what makes the DNS name `db-service.backend.svc.cluster.local` resolve.

```
Port 5432  → postgres container  (for psql debugging)
Port 50051 → db-service container (gRPC)
```

---

## redis

**Manifests**: `k8s/redis/`

No Secret or PVC — Redis is used as an ephemeral cache only.

### Deployment (`deployment.yaml`)

**redis container** (image: `redis:7`):
- Readiness probe: `redis-cli ping` (every 5s, initial delay 5s)
- No persistence flags — data is lost on pod restart (intentional)

**cache-service container** (image: `localhost:5000/cache-service:latest`):
- Connects to `localhost:6379`
- Gets `LOKI_URL` from env literal

### Service (`service.yaml`)

ClusterIP service named `cache-service`.

```
Port 6379  → redis container      (for redis-cli debugging)
Port 50052 → cache-service container (gRPC)
```

---

## loki

**Manifests**: `k8s/loki/`

### ConfigMap (`configmap.yaml`)

Loki configuration mounted at `/etc/loki/loki.yaml`. Key settings:

```yaml
auth_enabled: false          # No multi-tenancy
storage: filesystem          # Chunks + index on local PVC
replication_factor: 1        # Single node
schema: v13 / tsdb           # Current recommended schema
analytics.reporting: false   # No phone-home
```

### PVC (`pvc.yaml`)

```yaml
name: loki-pvc
size: 5Gi
accessMode: ReadWriteOnce
```

Mounted at `/loki` — Loki writes chunks to `/loki/chunks` and index to `/loki/index` (as configured).

### Deployment (`deployment.yaml`)

Single container (image: `grafana/loki:3.4.1`).

Probes:
- **Readiness**: GET `/ready` — initial delay 15s (Loki needs time to load TSDB index)
- **Liveness**: GET `/ready` — initial delay 30s

### Service (`service.yaml`)

ClusterIP named `loki` — DNS: `loki.backend.svc.cluster.local:3100`.

All three Rust services push logs to `http://loki.backend.svc.cluster.local:3100/loki/api/v1/push`.

---

## grafana

**Manifests**: `k8s/grafana/`

### ConfigMap (`configmap.yaml`)

Datasource provisioning file mounted at `/etc/grafana/provisioning/datasources/`. Grafana auto-loads this on startup, creating a Loki datasource pointing to `http://loki.backend.svc.cluster.local:3100`.

### Deployment (`deployment.yaml`)

Single container (image: `grafana/grafana:11.4.0`).

Environment variables:

| Variable | Value | Effect |
|----------|-------|--------|
| `GF_AUTH_ANONYMOUS_ENABLED` | `true` | Anyone can view dashboards without login |
| `GF_AUTH_ANONYMOUS_ORG_ROLE` | `Viewer` | Read-only access |
| `GF_AUTH_DISABLE_LOGIN_FORM` | `false` | Login form still available for admin |
| `GF_SECURITY_ALLOW_EMBEDDING` | `true` | Allows loading in `<iframe>` |
| `GF_SERVER_DOMAIN` | `localhost` | Used for redirect URLs |

Readiness probe: GET `/api/health` (initial delay 10s).

### Service (`service.yaml`)

NodePort — exposed externally on port **30300**.

```
External port 30300  →  container port 3000
```

Access: `http://<node-ip>:30300`

---

## experiproduct

**Manifests**: `k8s/experiproduct/`

### Secret (`secret.yaml`)

Contains credentials that must be configured before deployment:

| Key | Description |
|-----|-------------|
| `GOOGLE_CLIENT_ID` | From Google Cloud Console — OAuth 2.0 Client ID |
| `GOOGLE_CLIENT_SECRET` | From Google Cloud Console |
| `JWT_SECRET` | Random string, minimum 32 characters |

**Edit this file before first deploy:**
```bash
$EDITOR backend/k8s/experiproduct/secret.yaml
```

Or override with kubectl:
```bash
kubectl create secret generic experiproduct-secret -n backend \
  --from-literal=GOOGLE_CLIENT_ID=<id> \
  --from-literal=GOOGLE_CLIENT_SECRET=<secret> \
  --from-literal=JWT_SECRET=$(openssl rand -hex 32) \
  --dry-run=client -o yaml | kubectl apply -f -
```

### Deployment (`deployment.yaml`)

Two containers in one pod:

**product-backend container** (image: `localhost:5000/product-backend:latest`):
- Probes:
  - Readiness: GET `/api/health` (delay 10s, interval 10s)
  - Liveness: GET `/api/health` (delay 20s, interval 30s)
- Gets `JWT_SECRET`, `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET` from the Secret
- Gets `DB_SERVICE_URL`, `CACHE_SERVICE_URL`, `LOKI_URL` from env literals

**nginx container** (image: `localhost:5000/product-frontend:latest`):
- Serves the built React bundle
- Proxies `/api/*` to `localhost:8080` (product-backend, same pod)
- Readiness probe: GET `/` (delay 5s, interval 10s)

### Service (`service.yaml`)

NodePort — exposed externally on port **30080**.

```
External port 30080  →  container port 80  (nginx)
```

Access: `http://<node-ip>:30080`

---

## registry (local Docker registry)

**Manifests**: `k8s/registry/`

A `registry:2` deployment in the cluster, plus a Docker container on the host at `localhost:5000`. The k3s nodes are configured to trust `localhost:5000` as an insecure registry via `/etc/rancher/k3s/registries.yaml`.

This allows `docker push localhost:5000/my-image:tag` from the host and `image: localhost:5000/my-image:tag` in pod specs.

NodePort **30500** is also exposed for direct access.

---

## Storage Summary

| PVC | Deployment | Size | Path | Durable? |
|-----|-----------|------|------|---------|
| `postgresql-pvc` | postgresql | 5Gi | `/var/lib/postgresql/data` | Yes |
| `loki-pvc` | loki | 5Gi | `/loki` | Yes |

Redis has no PVC — data is ephemeral and rebuilt automatically by the heartbeat background task.

---

## Networking Summary

| Service | Type | Internal DNS | External Port |
|---------|------|-------------|---------------|
| `db-service` | ClusterIP | `db-service.backend.svc.cluster.local:50051` | — |
| `cache-service` | ClusterIP | `cache-service.backend.svc.cluster.local:50052` | — |
| `loki` | ClusterIP | `loki.backend.svc.cluster.local:3100` | — |
| `grafana` | NodePort | `grafana.backend.svc.cluster.local:3000` | **30300** |
| `experiproduct` | NodePort | `experiproduct.backend.svc.cluster.local:80` | **30080** |
| `registry` | NodePort | `registry.backend.svc.cluster.local:5000` | **30500** |

---

## Useful kubectl Commands

```bash
# All pods in the backend namespace
kubectl get pods -n backend

# Watch pod status in real-time
kubectl get pods -n backend -w

# Logs from a specific container
kubectl logs -n backend deployment/experiproduct -c product-backend
kubectl logs -n backend deployment/postgresql -c db-service
kubectl logs -n backend deployment/redis -c cache-service

# Describe a pod (events, resource limits, etc.)
kubectl describe pod -n backend -l app=experiproduct

# Execute a command inside a container (debug)
kubectl exec -n backend deployment/postgresql -c postgres -- \
  psql -U experiproduct -d experiproduct -c "SELECT * FROM items;"

kubectl exec -n backend deployment/redis -c redis -- redis-cli keys '*'

# Port-forward Grafana locally (if NodePort is blocked)
kubectl port-forward -n backend svc/grafana 3000:3000

# Check service endpoints
kubectl get endpoints -n backend
```
