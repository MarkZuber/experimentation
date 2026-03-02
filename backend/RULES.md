# backend/RULES.md — Code Conventions

## Rust

- **Edition**: 2021
- **Error handling**: `anyhow::Result` for main/top-level, `thiserror` for library errors
- **Async runtime**: `tokio` with `#[tokio::main]` or `#[actix_web::main]`
- **Naming**: `snake_case` for variables, functions, modules; `PascalCase` for types
- **Imports**: group std, external, internal; no wildcard imports except in tests
- **Logging**: use `tracing::info!`, `tracing::error!`, `tracing::warn!` — never `println!` in service code
- **gRPC**: tonic 0.12 for all inter-service communication
- **HTTP**: actix-web 4 only in product-backend; no HTTP in db-service or cache-service
- **Database**: sqlx 0.8 with compile-time checked queries where possible

## Proto / gRPC

- One `.proto` file per service under `backend/proto/`
- Package names match service names (`package items`, `package cache`)
- All RPC methods have distinct Request/Response message types
- Field names: `snake_case`

## TypeScript / React

- **Naming**: `camelCase` for variables/functions, `PascalCase` for components/types
- **State**: React hooks only (`useState`, `useEffect`, `useRef`)
- **Routing**: React Router v6 with `<Routes>` + `<Route>`
- **Auth guard**: Wrap protected routes in `<AuthGuard>` component
- **API calls**: Use `fetch` directly — no axios
- **Styling**: Inline styles with `React.CSSProperties` — no CSS files unless unavoidable
- **JWT**: Store in `localStorage` as `jwt`, send as `Authorization: Bearer <token>`

## Docker

- All Rust services: multi-stage `rust:1.83` → `debian:bookworm-slim`
- Frontend: multi-stage `node:22` → `nginx:alpine`
- Build context: always from `backend/` directory

## Kubernetes

- All resources in `backend` namespace
- Secrets for credentials (never ConfigMaps for sensitive data)
- Use `readinessProbe` on all containers
- Service names match the gRPC hostname used by clients:
  - `db-service.backend.svc.cluster.local:50051`
  - `cache-service.backend.svc.cluster.local:50052`
  - `loki.backend.svc.cluster.local:3100`
- NodePort ranges: app 30080, grafana 30300, registry 30500

## Scripts

- All scripts: `#!/usr/bin/env bash` + `set -euo pipefail`
- Idempotent: check before installing/creating
- Log prefix: `[script-name] message`
