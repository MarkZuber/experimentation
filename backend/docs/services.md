# Services

## Cargo Workspace

All three Rust services share a single Cargo workspace rooted at `backend/Cargo.toml`. Common dependencies (tokio, tonic, prost, tracing, etc.) are declared once in `[workspace.dependencies]` and referenced with `{ workspace = true }` in each member crate.

```
backend/
├── Cargo.toml          ← workspace root
├── db-service/         ← member crate
├── cache-service/      ← member crate
└── product-backend/    ← member crate
```

Build the whole workspace:
```bash
cd backend
cargo build --release
```

Build a single crate:
```bash
cargo build --release -p db-service
```

---

## Proto Definitions

Located in `backend/proto/`. Each Rust service has a `build.rs` that calls `tonic_build::compile_protos` to generate Rust code from these files at compile time.

### items.proto

Defines the **Items** service — full CRUD for an `Item` entity (id, name, description, timestamps).

```protobuf
service Items {
  rpc CreateItem(CreateItemRequest) returns (ItemResponse);
  rpc GetItem(GetItemRequest)       returns (ItemResponse);
  rpc ListItems(ListItemsRequest)   returns (ListItemsResponse);
  rpc UpdateItem(UpdateItemRequest) returns (ItemResponse);
  rpc DeleteItem(DeleteItemRequest) returns (DeleteItemResponse);
}
```

`db-service` compiles this as a **server**. `product-backend` compiles it as a **client**.

### cache.proto

Defines the **Cache** service — key/value operations against Redis, with optional TTL on writes.

```protobuf
service Cache {
  rpc SetKey(SetKeyRequest)     returns (SetKeyResponse);
  rpc GetKey(GetKeyRequest)     returns (GetKeyResponse);
  rpc DeleteKey(DeleteKeyRequest) returns (DeleteKeyResponse);
  rpc ListKeys(ListKeysRequest) returns (ListKeysResponse);
}
```

`SetKeyRequest` includes a `ttl_seconds` field (0 = no expiry). `ListKeysRequest` includes a `pattern` field for Redis glob-style filtering (default `*`).

`cache-service` compiles this as a **server**. `product-backend` compiles it as a **client**.

---

## db-service

**Location**: `backend/db-service/`
**Port**: `50051` (gRPC)
**Runs in**: `postgresql` pod alongside `postgres:15`

### What it does

Implements the `Items` gRPC service. On startup it:
1. Initializes Loki log shipping
2. Opens a `PgPool` connection to PostgreSQL on `localhost:5432`
3. Creates the `items` table if it doesn't exist:
   ```sql
   CREATE TABLE IF NOT EXISTS items (
       id          TEXT PRIMARY KEY,
       name        TEXT NOT NULL,
       description TEXT,
       created_at  TEXT NOT NULL,
       updated_at  TEXT NOT NULL
   )
   ```
4. Starts the gRPC server on `0.0.0.0:50051`

Item IDs are generated as UUID v4 strings. Timestamps are RFC 3339 strings.

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `postgres://postgres:postgres@localhost:5432/experiproduct` | PostgreSQL connection string |
| `LOKI_URL` | `http://loki:3100` | Loki base URL (path `/loki/api/v1/push` is appended) |
| `RUST_LOG` | *(unset)* | Tracing filter, e.g. `info`, `debug` |
| `HOSTNAME` | *(auto)* | Pod name, attached as a Loki label |

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `tonic` 0.12 | gRPC server |
| `prost` 0.13 | Protobuf encoding |
| `sqlx` 0.8 | Async PostgreSQL driver |
| `uuid` 1 | v4 ID generation |
| `chrono` 0.4 | Timestamp formatting |
| `tracing-loki` 0.2 | Log push to Loki |
| `tokio` 1 | Async runtime |

---

## cache-service

**Location**: `backend/cache-service/`
**Port**: `50052` (gRPC)
**Runs in**: `redis` pod alongside `redis:7`

### What it does

Implements the `Cache` gRPC service. On startup it:
1. Initializes Loki log shipping
2. Creates a `redis::Client` connected to `localhost:6379`
3. Starts the gRPC server on `0.0.0.0:50052`

Each gRPC call creates a `MultiplexedConnection` from the client for that request. `SetKey` uses `SET EX` when `ttl_seconds > 0`, otherwise plain `SET`. `ListKeys` fetches all keys matching the pattern and then fetches each value individually.

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `REDIS_URL` | `redis://localhost:6379` | Redis connection URL |
| `LOKI_URL` | `http://loki:3100` | Loki base URL |
| `RUST_LOG` | *(unset)* | Tracing filter |
| `HOSTNAME` | *(auto)* | Pod name label |

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `tonic` 0.12 | gRPC server |
| `redis` 0.27 | Async Redis client (`tokio-comp` feature) |
| `tracing-loki` 0.2 | Log push to Loki |
| `tokio` 1 | Async runtime |

---

## product-backend

**Location**: `backend/product-backend/`
**Port**: `8080` (HTTP)
**Runs in**: `experiproduct` pod alongside nginx

### What it does

The HTTP API gateway. On startup it:
1. Initializes Loki log shipping
2. Opens gRPC channels to `db-service` and `cache-service`
3. Wraps clients in `Arc<Mutex<>>` for shared use across requests
4. Spawns a background tokio task that writes `heartbeat:<n>` keys to Redis every 5 seconds (TTL 180s)
5. Starts the actix-web HTTP server on `0.0.0.0:8080`

### HTTP API

All routes under `/api/`. Protected routes require `Authorization: Bearer <jwt>` header.

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/health` | No | Liveness probe — returns `{"status":"ok"}` |
| `POST` | `/api/auth/google` | No | Exchange Google `id_token` for JWT |
| `GET` | `/api/items` | Yes | List all items |
| `POST` | `/api/items` | Yes | Create an item — body: `{"name":"...","description":"..."}` |
| `GET` | `/api/items/:id` | Yes | Get a single item |
| `PUT` | `/api/items/:id` | Yes | Update an item — body: `{"name":"...","description":"..."}` |
| `DELETE` | `/api/items/:id` | Yes | Delete an item |
| `GET` | `/api/redis` | Yes | List all Redis k/v pairs (pattern `*`) |

### JWT

JWTs are signed with HS256 using `JWT_SECRET`. The payload contains:

```json
{ "sub": "<google-sub>", "email": "<user-email>", "exp": <unix-timestamp> }
```

Expiry is 24 hours from issuance. The `extract_jwt` helper decodes and validates the token on every protected endpoint call.

### CORS

`actix-cors` is configured with `Cors::permissive()` — all origins and methods are allowed. Tighten this for production.

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DB_SERVICE_URL` | `http://localhost:50051` | gRPC endpoint for db-service |
| `CACHE_SERVICE_URL` | `http://localhost:50052` | gRPC endpoint for cache-service |
| `JWT_SECRET` | `dev-secret-change-me` | HS256 signing secret — **change this** |
| `GOOGLE_CLIENT_ID` | *(empty)* | OAuth2 client ID — required for auth to work |
| `GOOGLE_CLIENT_SECRET` | *(empty)* | OAuth2 client secret (stored but not currently used directly) |
| `LOKI_URL` | `http://loki:3100` | Loki base URL |
| `RUST_LOG` | *(unset)* | Tracing filter |
| `HOSTNAME` | *(auto)* | Pod name label |

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `actix-web` 4 | HTTP framework |
| `actix-cors` 0.7 | CORS middleware |
| `tonic` 0.12 | gRPC clients |
| `jsonwebtoken` 9 | JWT sign / verify |
| `reqwest` 0.12 | HTTP client (Google tokeninfo) |
| `tracing-loki` 0.2 | Log push to Loki |
| `tokio` 1 | Async runtime |

---

## product-frontend

**Location**: `backend/product-frontend/`
**Not a Cargo workspace member** — Node.js project.

### Stack

- **Vite** — build tool and dev server
- **React 18** — UI framework
- **TypeScript** — strict mode enabled
- **React Router v6** — client-side routing
- **`@react-oauth/google`** — Google One Tap and sign-in button
- **`plantuml-encoder`** — encodes PlantUML text for the plantuml.com renderer

### Pages

| Route | Component | Auth Required | Description |
|-------|-----------|:---:|-------------|
| `/` | `WelcomePage` | No | Google sign-in. Redirects to `/items` if already authenticated. |
| `/items` | `ItemsPage` | Yes | Full CRUD table — inline editing, delete with confirmation. |
| `/redis` | `RedisPage` | Yes | Live Redis k/v viewer, polls `/api/redis` every 5 seconds. |
| `/telemetry` | `TelemetryPage` | Yes | Grafana embedded in an `<iframe>` pointing to port 3000. |
| `/architecture` | `ArchitecturePage` | No | PlantUML diagram rendered via plantuml.com as an `<img>`. |

### Auth Guard

`AuthGuard` in `App.tsx` wraps protected routes. It checks for a `jwt` key in `localStorage`; if missing, it redirects to `/`.

### Build-Time Configuration

The Google Client ID must be baked into the frontend bundle at build time via a Vite env variable:

```bash
VITE_GOOGLE_CLIENT_ID=your-client-id npm run build
```

In the Docker build, `build-deploy.sh` reads this value from the k3s secret and passes it as a `--build-arg`.

### API Calls

All calls use the native `fetch` API. The JWT is read from `localStorage` and attached as an `Authorization: Bearer` header. No axios or other HTTP libraries are used.

---

## Grafana + Loki (Observability)

Loki and Grafana are third-party images — no custom code.

### Loki

Image: `grafana/loki:3.4.1`
Config: mounted from `ConfigMap` at `/etc/loki/loki.yaml`

Key config choices:
- `auth_enabled: false` — no per-tenant authentication
- Filesystem backend — chunks and index stored on the Loki PVC
- In-memory results cache (100 MB)
- TSDB schema v13 (current recommended schema)

Push endpoint used by Rust services: `http://loki.backend.svc.cluster.local:3100/loki/api/v1/push`

### Grafana

Image: `grafana/grafana:11.4.0`
Datasource: provisioned from `ConfigMap` at startup — points to Loki.

Anonymous access is enabled (`GF_AUTH_ANONYMOUS_ENABLED=true`) so the frontend iframe loads without requiring a Grafana login. Embedding is enabled via `GF_SECURITY_ALLOW_EMBEDDING=true`.

To query logs in Grafana:
1. Navigate to `http://<node-ip>:30300`
2. Open Explore
3. Select the **Loki** datasource
4. Filter by label, e.g. `{service="product-backend"}`
