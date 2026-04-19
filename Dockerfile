FROM rust:1.94-bullseye AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl gnupg \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
COPY frontend ./frontend
RUN cargo build --release --locked

FROM ghcr.io/thespaghettidetective/ml_api:latest

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tini \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/cc2-openwebui /usr/local/bin/cc2-openwebui
COPY --from=builder /src/frontend/dist /usr/frontend/dist

RUN mkdir -p /work /work/data /work/snapshots \
    && printf '%s\n' \
    '#!/usr/bin/env sh' \
    'set -eu' \
    'export PORT="${PORT:-3333}"' \
    'gunicorn -w 1 -b 0.0.0.0:"${PORT}" --chdir /app server:app &' \
    'ML_PID=$!' \
    "trap 'kill \"\$ML_PID\" 2>/dev/null || true' INT TERM EXIT" \
    'cd /work' \
    '/usr/local/bin/cc2-openwebui' \
    > /usr/local/bin/start.sh

RUN chmod +x /usr/local/bin/start.sh

WORKDIR /work
EXPOSE 8484 3333
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/usr/local/bin/start.sh"]
