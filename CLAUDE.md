# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Purpose

This is an experimentation repo containing setup scripts for a Ubuntu development environment.

## Structure

- `scripts/ubuntu_setup.sh` — Idempotent Ubuntu dev environment setup script

## What the Setup Script Does

The script sets up a complete Ubuntu dev environment in this order:

1. Installs apt packages (git, zsh, neovim, tmux, fzf, ripgrep, bat, lsd, fastfetch, gcc, make, cmake, git-delta, dotnet-sdk-10.0)
2. Clones dotfiles from `github.com/markzuber/dotfiles`
3. Installs Oh-My-Zsh with plugins: `zsh-autosuggestions`, `zsh-syntax-highlighting`, `fzf-tab`, and Powerlevel10k theme
4. Installs Git Credential Manager (manual `.deb` download from GitHub releases)
5. Installs UV (Python toolchain manager via `astral.sh`)
6. Installs Node.js via nvm with global packages: typescript, ts-node, prettier, eslint
7. Symlinks dotfiles (zsh, ghostty, p10k, nvim, tmux, editorconfig, gitconfig)
8. Installs Rust via rustup
9. Installs Claude Code

## Running the Script

```bash
bash scripts/ubuntu_setup.sh
```

The script is idempotent — it checks for existing installations before reinstalling. Run it on a fresh Ubuntu system.

---

## Backend: ExperiProduct Kubernetes App

A full-stack Rust + React application deployed on k3s.

### Structure

```
backend/
├── Cargo.toml              (workspace)
├── proto/                  (gRPC proto definitions)
├── db-service/             (Rust: tonic gRPC + sqlx → PostgreSQL)
├── cache-service/          (Rust: tonic gRPC + redis-rs → Redis)
├── product-backend/        (Rust: actix-web HTTP API + tonic clients)
├── product-frontend/       (Vite + React + TypeScript)
├── k8s/                    (Kubernetes manifests)
├── docker/                 (Multi-stage Dockerfiles)
└── scripts/                (setup-k8s.sh, build-deploy.sh, uninstall.sh)
```

### Quick Start

```bash
# 1. Install k3s, Docker, protoc, local registry
bash backend/scripts/setup-k8s.sh

# 2. Configure secrets (Google OAuth + JWT)
$EDITOR backend/k8s/experiproduct/secret.yaml

# 3. Build all images and deploy to k3s
bash backend/scripts/build-deploy.sh

# 4. Teardown
bash backend/scripts/uninstall.sh
```

### Build Commands

```bash
# Build Rust workspace
cd backend && cargo build --release

# Build frontend only
cd backend/product-frontend && npm ci && npm run build

# Build a specific Docker image (run from backend/)
docker build -f docker/db-service.Dockerfile -t localhost:5000/db-service:latest .
```

### Pods (all in `backend` namespace)

| Deployment | Containers | Exposed |
|-----------|-----------|---------|
| postgresql | postgres:15 + db-service (gRPC 50051) | ClusterIP |
| redis | redis:7 + cache-service (gRPC 50052) | ClusterIP |
| loki | grafana/loki:3.4 | ClusterIP 3100 |
| grafana | grafana/grafana:11 | NodePort 30300 |
| experiproduct | product-backend (8080) + nginx (80) | NodePort 30080 |

### Access (after deploy)

- App: `http://<node-ip>:30080`
- Grafana: `http://<node-ip>:30300`
