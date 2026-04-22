# Go-Wails Headless Deployment Guide

This guide covers production deployment for the `go-wails` headless service using Docker and Docker Compose.

## 1. Prerequisites

- Docker engine installed and running
- Docker Compose v2 available (`docker compose`)
- Repository checked out on `main`

## 2. Required Environment

The root `docker-compose.yml` reads:
- `./.env.example` (required defaults)
- `./.env` (optional override, recommended for real keys)

Create or update root `.env`:

```bash
rtk cp .env.example .env
```

Minimum keys to set for real deployment:
- `AUTH_TOKEN`
- `ALLOWED_TOKENS`
- `API_KEYS`

Recommended runtime settings:
- `RUNTIME_BIND_HOST=0.0.0.0`
- `RUNTIME_PORT_START=18080`
- `RUNTIME_PORT_END=18080`

## 3. Build and Start

From repository root:

```bash
rtk docker compose build go-wails-headless
rtk docker compose up -d go-wails-headless
```

Check status:

```bash
rtk docker compose ps
rtk docker compose logs --tail=200 go-wails-headless
```

## 4. Health and API Verification

```bash
rtk curl -sS http://127.0.0.1:18080/api/v1/health
rtk curl -sS http://127.0.0.1:18080/v1/models -H "Authorization: Bearer sk-user-demo"
```

Phase 2 smoke script:

```bash
cd go-wails
rtk bash scripts/smoke-phase2.sh
cd ..
```

## 5. Upgrade Procedure

```bash
rtk git pull --ff-only
rtk docker compose build go-wails-headless
rtk docker compose up -d go-wails-headless
rtk docker compose logs --tail=200 go-wails-headless
```

## 6. Rollback Procedure

Rollback to previous git commit:

```bash
rtk git checkout <previous_commit_or_tag>
rtk docker compose build go-wails-headless
rtk docker compose up -d go-wails-headless
```

Then verify health endpoints again.

## 7. Stop / Remove

Stop service:

```bash
rtk docker compose stop go-wails-headless
```

Stop and remove container/network:

```bash
rtk docker compose down
```

