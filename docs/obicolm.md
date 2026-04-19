# Obico ML quick guide

If Obico ML test fails in onboarding, check this.

## 1. Normal case

You should not need extra setup: the main project Dockerfile is supposed to run Obico ML with the app.

## 2. Manual run

If needed, run Obico ML alone to test:

```bash
docker run -d --name obico-ml --restart unless-stopped --network host ghcr.io/thespaghettidetective/ml_api:latest
```

Then test from host:

```bash
curl -I http://127.0.0.1:3333/p/
```

Any HTTP response means the service is reachable.

## 3. Common issues

- Docker daemon not running
- Port `3333` already used by another process
- Container exited after start
- Wrong URL in onboarding (must point to Obico ML endpoint)

## 4. Fast diagnostics

```bash
docker ps -a | grep obico-ml
docker logs --tail=100 obico-ml
ss -ltnp | grep 3333
```

If the container is healthy and port is open, set URL in onboarding and run **Test Obico ML connection** again.
