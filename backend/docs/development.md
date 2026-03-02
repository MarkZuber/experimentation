# Local Development

Run the backend services and frontend locally without k3s or Docker. Useful for fast iteration — Rust rebuilds and Vite hot-module replacement work as normal.

## Prerequisites

- Rust (stable, via [rustup](https://rustup.rs))
- `protoc` — Protocol Buffer compiler
  ```bash
  sudo apt install protobuf-compiler        # Ubuntu
  brew install protobuf                     # macOS
  ```
- Node.js 20+ and npm
- Docker (optional, for running PostgreSQL and Redis as containers)
- PostgreSQL 15 running locally
- Redis 7 running locally

### Start PostgreSQL and Redis with Docker

The fastest way to get datastores running locally:

```bash
docker run -d --name dev-postgres \
  -e POSTGRES_USER=experiproduct \
  -e POSTGRES_PASSWORD=experiproduct-pg-pass \
  -e POSTGRES_DB=experiproduct \
  -p 5432:5432 \
  postgres:15

docker run -d --name dev-redis \
  -p 6379:6379 \
  redis:7
```

Stop and remove when done:
```bash
docker rm -f dev-postgres dev-redis
```

---

## Running the Rust Services

Each service is a standalone binary. Open three terminal windows.

### Terminal 1 — db-service

```bash
cd backend
DATABASE_URL=postgres://experiproduct:experiproduct-pg-pass@localhost:5432/experiproduct \
LOKI_URL=http://localhost:3100 \
RUST_LOG=info \
cargo run -p db-service
```

Output:
```
INFO db_service: Connecting to database: postgres://experiproduct:...
INFO db_service: Database initialized
INFO db_service: db-service listening on 0.0.0.0:50051
```

### Terminal 2 — cache-service

```bash
cd backend
REDIS_URL=redis://localhost:6379 \
LOKI_URL=http://localhost:3100 \
RUST_LOG=info \
cargo run -p cache-service
```

Output:
```
INFO cache_service: Connecting to Redis: redis://localhost:6379
INFO cache_service: cache-service listening on 0.0.0.0:50052
```

### Terminal 3 — product-backend

```bash
cd backend
DB_SERVICE_URL=http://localhost:50051 \
CACHE_SERVICE_URL=http://localhost:50052 \
JWT_SECRET=dev-local-secret-change-in-prod \
GOOGLE_CLIENT_ID=your-google-client-id.apps.googleusercontent.com \
LOKI_URL=http://localhost:3100 \
RUST_LOG=info \
cargo run -p product-backend
```

Output:
```
INFO product_backend: Connecting to db-service: http://localhost:50051
INFO product_backend: Connecting to cache-service: http://localhost:50052
INFO product_backend: product-backend listening on 0.0.0.0:8080
```

> `LOKI_URL` can point to a non-existent URL locally — `tracing-loki` will retry silently. Set it to any reachable Loki if you want to capture local logs.

---

## Running the Frontend

```bash
cd backend/product-frontend
npm install
VITE_GOOGLE_CLIENT_ID=your-google-client-id.apps.googleusercontent.com npm run dev
```

Vite starts a dev server on `http://localhost:5173` with hot-module replacement. API calls to `/api/*` are proxied to `http://localhost:8080` (the product-backend running in Terminal 3).

The proxy is configured in `vite.config.ts`:
```typescript
server: {
  proxy: {
    '/api': 'http://localhost:8080',
  },
},
```

---

## Building

### All Rust services

```bash
cd backend
cargo build --release
```

Binaries are placed at:
- `backend/target/release/db-service`
- `backend/target/release/cache-service`
- `backend/target/release/product-backend`

### Frontend

```bash
cd backend/product-frontend
npm run build
```

Output goes to `backend/product-frontend/dist/`.

### Type checking only (no emit)

```bash
cd backend/product-frontend
npx tsc --noEmit
```

---

## Proto Codegen

Proto code is generated automatically during `cargo build` via `build.rs`. There's nothing to run manually. To force regeneration, touch the proto files:

```bash
touch backend/proto/items.proto backend/proto/cache.proto
cargo build -p db-service
```

Generated code is written to `target/` — never edit it directly.

---

## Testing the API Manually

With all services running, use `curl` to hit the endpoints.

### Health check
```bash
curl http://localhost:8080/api/health
# {"status":"ok"}
```

### Auth (requires a real Google ID token from the browser)
```bash
# Get an id_token from the browser console after signing in:
# googleAuth.currentUser.get().getAuthResponse().id_token
curl -X POST http://localhost:8080/api/auth/google \
  -H 'Content-Type: application/json' \
  -d '{"id_token": "<token-from-browser>"}'
# {"token":"eyJ..."}
```

### Items CRUD
```bash
TOKEN="eyJ..."   # JWT from auth step

# List items
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/items

# Create item
curl -X POST http://localhost:8080/api/items \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"name":"test","description":"hello"}'

# Update item
curl -X PUT http://localhost:8080/api/items/<id> \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"name":"updated","description":"world"}'

# Delete item
curl -X DELETE -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/items/<id>
```

### Redis keys
```bash
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/redis
```

---

## Directory Structure for Development

When adding new features, the typical files to touch are:

| Change | Files |
|--------|-------|
| New gRPC method | `proto/*.proto`, `db-service/src/main.rs` or `cache-service/src/main.rs`, `product-backend/src/main.rs` |
| New HTTP endpoint | `product-backend/src/main.rs` |
| New frontend page | `product-frontend/src/pages/<Name>Page.tsx`, `product-frontend/src/App.tsx` |
| New database column | `proto/items.proto`, `db-service/src/main.rs` (SQL + handler), `product-backend/src/main.rs` (client) |

---

## Code Conventions

See [RULES.md](../RULES.md) for full conventions. Key points:

- Rust: `snake_case`, `anyhow::Result` for errors, `tracing::info!` not `println!`
- TypeScript: `camelCase` variables, `PascalCase` components, inline styles only, native `fetch`
- All new gRPC methods need Request and Response wrapper types in proto
- Protected HTTP endpoints must call `extract_jwt` and return early on error
