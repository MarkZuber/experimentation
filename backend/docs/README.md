# ExperiProduct Documentation

ExperiProduct is a full-stack Rust + React application deployed on [k3s](https://k3s.io/) (lightweight Kubernetes) on a single Ubuntu machine. It demonstrates a microservices architecture with gRPC for inter-service communication, PostgreSQL and Redis for storage, and Loki + Grafana for observability.

## Documentation Index

| Doc | Description |
|-----|-------------|
| [architecture.md](architecture.md) | System overview, component diagram, data flow, communication patterns |
| [services.md](services.md) | Each service in detail — APIs, env vars, dependencies |
| [kubernetes.md](kubernetes.md) | K8s manifests explained — pods, services, storage, networking |
| [deployment.md](deployment.md) | Step-by-step: install, configure, build, deploy, tear down |
| [development.md](development.md) | Local development without k3s |

## Quick Reference

```
App URL:     http://<node-ip>:30080
Grafana:     http://<node-ip>:30300
```

```bash
# First-time setup
bash backend/scripts/setup-k8s.sh

# Build and deploy
bash backend/scripts/build-deploy.sh

# Tear down
bash backend/scripts/uninstall.sh
```

## Stack at a Glance

| Layer | Technology |
|-------|-----------|
| Frontend | React 18 + TypeScript + Vite |
| HTTP API | Rust + actix-web 4 |
| Inter-service comms | gRPC (tonic 0.12) |
| Database | PostgreSQL 15 (sqlx 0.8) |
| Cache | Redis 7 (redis-rs 0.27) |
| Logging | tracing-loki → Loki 3.4 → Grafana 11 |
| Auth | Google OAuth 2.0 → JWT (HS256) |
| Container runtime | k3s with Docker |
| Local image registry | registry:2 on localhost:5000 |
