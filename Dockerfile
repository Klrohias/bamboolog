FROM node:22-alpine AS frontend
WORKDIR /app

COPY bamboolog-admin/package.json bamboolog-admin/pnpm-lock.yaml ./
RUN --mount=type=cache,id=pnpm-store,target=/pnpm/store \
    corepack enable && \
    pnpm config set store-dir /pnpm/store && \
    pnpm fetch --frozen-lockfile

COPY bamboolog-admin/ ./
RUN --mount=type=cache,id=pnpm-store,target=/pnpm/store \
    pnpm config set store-dir /pnpm/store && \
    pnpm install --offline --frozen-lockfile --dangerously-allow-all-builds && \
    pnpm run build

FROM rust:1.94-alpine3.22 AS builder
WORKDIR /app

RUN apk add --no-cache build-base cmake musl-dev

# Copy manifests first so dependency downloads remain cached when source changes.
COPY Cargo.toml Cargo.lock ./
COPY bamboolog/Cargo.toml bamboolog/Cargo.toml
COPY bamboolog/src/ bamboolog/src/
COPY --from=frontend /app/dist/ bamboolog/admin-dist/

RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=cargo-target,target=/app/target \
    cargo build --release --locked --package bamboolog --bin bamboolog && \
    cp target/release/bamboolog /usr/local/bin/bamboolog

FROM alpine:3.22 AS runtime
RUN apk add --no-cache ca-certificates libgcc su-exec && \
    addgroup -S bamboolog && \
    adduser -S -G bamboolog -h /app bamboolog

WORKDIR /app
COPY --from=builder /usr/local/bin/bamboolog /usr/local/bin/bamboolog
COPY docker/config.toml /usr/local/share/bamboolog/config.toml
COPY docker/entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod 755 /usr/local/bin/docker-entrypoint.sh

ENV CONFIG_PATH=/app/config.toml
VOLUME ["/app"]
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
