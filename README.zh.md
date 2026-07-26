# Bamboolog

[English](README.md)

Bamboolog 是一个使用 Rust 编写的自托管博客平台。它通过同一个应用提供公开博客和内嵌管理后台，并支持 SQLite、MySQL 与 PostgreSQL。

## 特性

- 服务端渲染的公开博客，支持 Markdown 文章、分类与标签、分页、RSS 和站点地图。
- 内嵌 Vue 管理后台，访问路径为 `/admin`，可管理文章、站点设置、主题、附件、存储引擎和个人资料。
- 可安装主题，支持主题独立配置与翻译。
- 支持本地存储和 S3 兼容对象存储作为附件存储后端。
- 管理后台使用基于 HTTP-only cookie 的会话，同时保留 Bearer Token API 兼容性。

## 快速开始

前置条件：

- 兼容 Rust 2024 edition 的 Rust 工具链。
- 仅在本地构建管理后台时需要 Node.js 22 与 Corepack/pnpm。

在仓库根目录创建 `config.toml`：

```toml
listen_addr = "127.0.0.1:8081"
database = "sqlite://./bamboolog.db?mode=rwc"
asset_dir = "."
```

初始化数据库并创建管理员账户：

```sh
cargo run -p bamboolog -- sync-entities-ef
cargo run -p bamboolog -- create-admin
```

启动服务：

```sh
cargo run -p bamboolog
```

公开站点地址为 `http://127.0.0.1:8081`，管理后台地址为 `http://127.0.0.1:8081/admin`。

> 服务端从 `bamboolog/admin-dist` 嵌入生产版管理后台。需要在本地构建时，请先完成[管理后台开发](#管理后台开发)。

## Docker

提供的 Compose 文件会将前端与后端构建为同一个镜像，将应用数据持久化到 `./app`，并暴露 `8081` 端口。

```sh
docker compose -f docker-compose.example.yml up --build -d
docker compose -f docker-compose.example.yml exec bamboolog sync-entities-ef
docker compose -f docker-compose.example.yml exec bamboolog create-admin
```

创建管理员账户后，访问 `http://127.0.0.1:8081/admin`。

容器首次启动时会从内置默认配置创建 `/app/config.toml`。修改持久化的 [`app/config.toml`](app/config.toml) 可以调整监听地址、数据库连接和资源目录；修改后重启容器即可生效。

## 配置

`CONFIG_PATH` 用于指定静态配置文件，默认读取当前目录的 `config.toml`。

| 配置项 | 说明 |
| --- | --- |
| `listen_addr` | HTTP 服务监听地址，例如 `0.0.0.0:8081`。 |
| `database` | SeaORM 数据库 URL。SQLite 是默认部署方案；MySQL 和 PostgreSQL 通过 Cargo feature 启用。 |
| `asset_dir` | 应用资源目录，包含已安装主题和本地附件存储。相对路径以配置文件所在目录为基准。 |

构建非 SQLite 部署时，选择对应的数据库 feature：

```sh
cargo run -p bamboolog --no-default-features --features postgres
# 或
cargo run -p bamboolog --no-default-features --features mysql
```

新建数据库后需要执行 `sync-entities-ef`。该命令会同步实体表结构，并创建默认的本地存储引擎。

## 管理后台开发

在一个终端启动 API 服务，在另一个终端启动 Vite：

```sh
cd bamboolog-admin
corepack enable
pnpm install --frozen-lockfile
pnpm dev
```

Vite 会将 `/api` 和 `/attachments` 代理到 `http://127.0.0.1:8081`。使用以下命令构建并嵌入管理后台：

```sh
pnpm build
rm -rf ../bamboolog/admin-dist
cp -R dist ../bamboolog/admin-dist
```

## 常用操作

```sh
# 同步实体表结构并创建默认存储引擎
cargo run -p bamboolog -- sync-entities-ef

# 交互式创建管理员账户
cargo run -p bamboolog -- create-admin

# 运行测试
cargo test -p bamboolog
```

通过 `RUST_LOG` 控制服务端日志级别，例如：

```sh
RUST_LOG=bamboolog=debug cargo run -p bamboolog
```

## 更多文档

- [附件存储](docs/storage.md)
- [主题配置](docs/theme-configuration.md)
- [主题与系统边界](docs/theme-system-boundaries.md)

## 项目结构

| 路径 | 用途 |
| --- | --- |
| `bamboolog/` | Rust 应用、HTTP API、公开站点渲染器及内嵌管理后台资源。 |
| `bamboolog-admin/` | Vue/Vite 管理后台。 |
| `docker/` | 容器配置与入口脚本。 |
| `docs/` | 运行与主题文档。 |

## 许可证

本项目基于 [MIT License](LICENSE) 发布。Copyright (c) 2026 Klrohias Dev。
