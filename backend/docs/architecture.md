# Architecture

## Overview

ExperiProduct is a microservices application where all backend logic is written in Rust. Three services communicate over gRPC; a React frontend talks only to the HTTP API. Everything runs in a single `backend` namespace on k3s.

```
                          ┌─────────────────────────────────────────────────────┐
                          │  k3s Cluster (single Ubuntu node)                   │
                          │                                                      │
  Browser ──HTTP:30080──► │  ┌──────────────────────────────┐                   │
                          │  │  experiproduct pod           │                   │
                          │  │  ┌───────┐   ┌────────────┐  │                   │
                          │  │  │ nginx │──►│ product-   │  │                   │
                          │  │  │ :80   │   │ backend    │  │                   │
                          │  │  └───────┘   │ :8080      │  │                   │
                          │  └─────────────-│────────────┘──┘                   │
                          │                 │         │                          │
                          │     gRPC:50051  │         │ gRPC:50052               │
                          │                 ▼         ▼                          │
                          │  ┌──────────────────┐  ┌──────────────────┐         │
                          │  │  postgresql pod  │  │  redis pod       │         │
                          │  │  ┌────────────┐  │  │  ┌────────────┐  │         │
                          │  │  │ db-service │  │  │  │cache-svc   │  │         │
                          │  │  │ :50051     │  │  │  │ :50052     │  │         │
                          │  │  └─────┬──────┘  │  │  └─────┬──────┘  │         │
                          │  │        │          │  │        │          │         │
                          │  │  ┌─────▼──────┐  │  │  ┌─────▼──────┐  │         │
                          │  │  │ postgres   │  │  │  │ redis:7    │  │         │
                          │  │  │ :5432      │  │  │  │ :6379      │  │         │
                          │  │  └────────────┘  │  │  └────────────┘  │         │
                          │  └──────────────────┘  └──────────────────┘         │
                          │                                                      │
                          │  ┌──────────────────┐  ┌──────────────────┐         │
                          │  │  loki pod        │  │  grafana pod     │         │
                          │  │  :3100           │◄─│  :3000           │         │
                          │  └──────────────────┘  └──────────────────┘         │
                          │          ▲                       │                   │
                          │          │ tracing-loki          │ NodePort:30300    │
                          │          │ (all 3 Rust svcs)     ▼                   │
                          └──────────┼──────────────── Browser:30300 ────────────┘
                                     │
                               Google OAuth2
                               (tokeninfo API)
```

## Pods

There are five pods (Deployments) in the `backend` namespace, each representing a logical unit:

| Deployment | Containers | Purpose |
|-----------|-----------|---------|
| `postgresql` | postgres:15, db-service | Persistent item storage |
| `redis` | redis:7, cache-service | In-memory caching + heartbeat |
| `loki` | grafana/loki:3.4 | Log aggregation |
| `grafana` | grafana/grafana:11 | Log visualization (NodePort 30300) |
| `experiproduct` | product-backend, nginx | HTTP API + static frontend (NodePort 30080) |

## Sidecar Pattern

The `db-service` and `cache-service` Rust binaries each run as **sidecars** in the same pod as their respective datastores. Because containers in a pod share the same network namespace, the Rust service connects to its datastore on `localhost` rather than over the cluster network:

- `db-service` → PostgreSQL on `localhost:5432`
- `cache-service` → Redis on `localhost:6379`

This avoids an extra Kubernetes Service hop for the high-frequency datastore connections.

The `product-backend` and `nginx` containers also share a pod. Nginx proxies `/api/*` requests to `localhost:8080` (product-backend), so only one NodePort is needed to serve both the static frontend and the API.

## gRPC Inter-Service Communication

`product-backend` reaches the two gRPC services via Kubernetes ClusterIP Services, using the internal DNS name:

```
db-service.backend.svc.cluster.local:50051     (Items CRUD)
cache-service.backend.svc.cluster.local:50052  (Cache k/v ops)
```

The gRPC channel is established once at startup; clients are wrapped in `Arc<Mutex<>>` and shared across actix-web request handlers.

```
product-backend  ──gRPC──►  db-service      ──sqlx──►  PostgreSQL
product-backend  ──gRPC──►  cache-service   ──redis──►  Redis
```

## Authentication Flow

```
  Browser                 product-backend           Google
     │                          │                      │
     │── POST /api/auth/google ──►                      │
     │   { id_token: "..." }    │── GET tokeninfo?id_token=... ──►│
     │                          │◄─── { sub, email, aud } ────────│
     │                          │  [verify aud == GOOGLE_CLIENT_ID]│
     │◄── { token: "<jwt>" } ───│                      │
     │                          │
     │── GET /api/items ──────► │
     │   Authorization: Bearer <jwt>
     │◄── [...items] ─────────  │
```

1. The browser receives a Google `id_token` via the `@react-oauth/google` One Tap SDK.
2. It posts this token to `POST /api/auth/google`.
3. `product-backend` verifies the token with Google's public tokeninfo endpoint, checking the `aud` field matches `GOOGLE_CLIENT_ID`.
4. On success, it signs and returns a short-lived JWT (HS256, 24h TTL) containing `sub` and `email` claims.
5. The frontend stores this JWT in `localStorage` and attaches it as `Authorization: Bearer <token>` on every subsequent API request.

## Observability (Loki + Grafana)

All three Rust services initialize `tracing-loki` at startup, which pushes structured log entries to Loki over HTTP:

```rust
let (loki_layer, task) = tracing_loki::builder()
    .label("service", "db-service")          // service label for filtering
    .extra_field("pod", env::var("HOSTNAME")) // pod name label
    .build_url(Url::parse(&loki_push_url))?;

tokio::spawn(task);  // background push task
```

Every `tracing::info!` / `tracing::error!` call is forwarded to `loki.backend.svc.cluster.local:3100`. Grafana is pre-configured with Loki as its default datasource, so logs are queryable immediately after deployment. The Telemetry page in the frontend embeds Grafana in an iframe.

## Background Heartbeat

`product-backend` spawns a background tokio task that writes a new Redis key every 5 seconds with a 180-second TTL:

```
heartbeat:1  →  "alive at 2026-03-02T10:00:00Z"
heartbeat:2  →  "alive at 2026-03-02T10:00:05Z"
...
```

This provides a live demonstration of the cache layer and ensures the Redis page in the frontend always has data to show.

## Persistent Storage

Two PersistentVolumeClaims (5Gi each) are created for data that must survive pod restarts:

| PVC | Mounted in | Path |
|-----|-----------|------|
| `postgresql-pvc` | postgres container | `/var/lib/postgresql/data` |
| `loki-pvc` | loki container | `/loki` |

Redis data is ephemeral (no PVC) — this is intentional given its role as a short-lived cache.

## Local Image Registry

Because k3s pulls images from registries, a local Docker registry (`registry:2`) runs on the host at `localhost:5000`. k3s is configured via `/etc/rancher/k3s/registries.yaml` to accept pushes and pulls from this insecure (HTTP) registry. The `build-deploy.sh` script tags images as `localhost:5000/<name>:latest` and pushes them before applying manifests.
