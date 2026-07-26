# Bamboolog

[简体中文](README.zh.md)

Bamboolog is a self-hosted blog platform written in Rust. It serves the public site and an embedded administration interface from one application, with SQLite, MySQL, and PostgreSQL support.

## Highlights

- Server-rendered public blog with Markdown posts, taxonomies, pagination, RSS, and sitemap support.
- Embedded Vue administration interface at `/admin` for posts, site settings, themes, attachments, storage engines, and user profile management.
- Installable themes with per-theme configuration and translations.
- Local or S3-compatible attachment storage.
- Cookie-based HTTP-only sessions for the administration interface, with bearer-token API compatibility.

## Quick Start

Prerequisites:

- Rust toolchain compatible with the 2024 edition.
- Node.js 22 and Corepack/pnpm, only when building the administration interface locally.

Create `config.toml` in the repository root:

```toml
listen_addr = "127.0.0.1:8081"
database = "sqlite://./bamboolog.db?mode=rwc"
asset_dir = "."
```

Initialize the database and create an administrator account:

```sh
cargo run -p bamboolog -- sync-entities-ef
cargo run -p bamboolog -- create-admin
```

Start the server:

```sh
cargo run -p bamboolog
```

Open the public site at `http://127.0.0.1:8081` and the administration interface at `http://127.0.0.1:8081/admin`.

> The server embeds the production administration build from `bamboolog/admin-dist`. For a local production build, follow [Administration Development](#administration-development) before running the server.

## Docker

The supplied Compose file builds the frontend and backend into one image, persists application data in `./app`, and exposes port `8081`.

```sh
docker compose -f docker-compose.example.yml up --build -d
docker compose -f docker-compose.example.yml exec bamboolog sync-entities-ef
docker compose -f docker-compose.example.yml exec bamboolog create-admin
```

Visit `http://127.0.0.1:8081/admin` after creating the administrator account.

The container creates `/app/config.toml` from the bundled default on first start. Update the persisted [`app/config.toml`](app/config.toml) to change the listen address, database connection, or asset directory, then restart the container.

## Configuration

`CONFIG_PATH` selects the static configuration file and defaults to `config.toml` in the current directory.

| Key | Description |
| --- | --- |
| `listen_addr` | Address the HTTP server binds to, for example `0.0.0.0:8081`. |
| `database` | SeaORM database URL. SQLite is the default deployment choice; MySQL and PostgreSQL are available through Cargo features. |
| `asset_dir` | Directory for application-owned assets, including installed themes and local attachment storage. Relative paths are resolved from the configuration file's directory. |

Use one database feature when building for a non-SQLite deployment:

```sh
cargo run -p bamboolog --no-default-features --features postgres
# or
cargo run -p bamboolog --no-default-features --features mysql
```

Run `sync-entities-ef` after setting up a new database. It synchronizes the entity schema and seeds the default local storage engine.

## Administration Development

Run the API server in one terminal. In another terminal, start Vite:

```sh
cd bamboolog-admin
corepack enable
pnpm install --frozen-lockfile
pnpm dev
```

Vite proxies `/api` and `/attachments` to `http://127.0.0.1:8081`. Build the embedded administration bundle with:

```sh
pnpm build
rm -rf ../bamboolog/admin-dist
cp -R dist ../bamboolog/admin-dist
```

## Operations

```sh
# Synchronize entities and seed the default storage engine
cargo run -p bamboolog -- sync-entities-ef

# Create an administrator interactively
cargo run -p bamboolog -- create-admin

# Run tests
cargo test -p bamboolog
```

Set `RUST_LOG` to control server logging, for example:

```sh
RUST_LOG=bamboolog=debug cargo run -p bamboolog
```

## Documentation

- [Attachment storage](docs/storage.md)
- [Theme configuration](docs/theme-configuration.md)
- [Theme and system boundaries](docs/theme-system-boundaries.md)

## Project Layout

| Path | Purpose |
| --- | --- |
| `bamboolog/` | Rust application, HTTP API, public-site renderer, and embedded admin assets. |
| `bamboolog-admin/` | Vue/Vite administration interface. |
| `docker/` | Container configuration and entrypoint. |
| `docs/` | Operational and theme documentation. |

## License

Distributed under the [MIT License](LICENSE). Copyright (c) 2026 Klrohias Dev.
