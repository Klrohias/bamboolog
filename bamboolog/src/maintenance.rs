use crate::entity::user;
use anyhow::Result;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, Set, Statement,
};
use std::io::{self, Write};

pub async fn sync_entities(database: &DatabaseConnection) -> Result<()> {
    tracing::info!("Sync entities (Entity first)");

    migrate_legacy_sqlite_schema(database).await?;

    database
        .get_schema_registry("bamboolog::entity::*")
        .sync(database)
        .await?;

    // Seed default storage engine
    use crate::entity::storage_engine;
    let internal_exists = storage_engine::Entity::find()
        .filter(storage_engine::Column::Kind.is_in(["local", "internal"]))
        .one(database)
        .await?;

    if internal_exists.is_none() {
        tracing::info!("Seeding default local storage engine");
        let active_model = storage_engine::ActiveModel {
            name: Set("Local Storage".to_string()),
            comments: Set("Default local storage".to_string()),
            kind: Set("local".to_string()),
            config_json: Set(None),
            is_default: Set(true),
            enabled: Set(true),
            ..Default::default()
        };
        active_model.insert(database).await?;
    }

    Ok(())
}

async fn migrate_legacy_sqlite_schema(database: &DatabaseConnection) -> Result<()> {
    if database.get_database_backend() != DbBackend::Sqlite {
        return Ok(());
    }

    migrate_legacy_storage_engines(database).await?;
    migrate_legacy_attachments(database).await?;
    Ok(())
}

async fn migrate_legacy_storage_engines(database: &DatabaseConnection) -> Result<()> {
    if !sqlite_has_column(database, "storage_engines", "type").await? {
        return Ok(());
    }

    if !sqlite_has_column(database, "storage_engines", "kind").await? {
        execute_sqlite(
            database,
            "ALTER TABLE storage_engines ADD COLUMN kind varchar NOT NULL DEFAULT 'local'",
        )
        .await?;
        execute_sqlite(database, "UPDATE storage_engines SET kind = type").await?;
    }
    if !sqlite_has_column(database, "storage_engines", "config_json").await? {
        execute_sqlite(
            database,
            "ALTER TABLE storage_engines ADD COLUMN config_json varchar",
        )
        .await?;
        execute_sqlite(database, "UPDATE storage_engines SET config_json = config").await?;
    }
    if !sqlite_has_column(database, "storage_engines", "is_default").await? {
        execute_sqlite(
            database,
            "ALTER TABLE storage_engines ADD COLUMN is_default boolean NOT NULL DEFAULT 0",
        )
        .await?;
        execute_sqlite(
            database,
            "UPDATE storage_engines SET is_default = 1 WHERE id = (SELECT id FROM storage_engines ORDER BY id LIMIT 1)",
        )
        .await?;
    }
    if !sqlite_has_column(database, "storage_engines", "enabled").await? {
        execute_sqlite(
            database,
            "ALTER TABLE storage_engines ADD COLUMN enabled boolean NOT NULL DEFAULT 1",
        )
        .await?;
    }

    Ok(())
}

async fn migrate_legacy_attachments(database: &DatabaseConnection) -> Result<()> {
    if !sqlite_has_column(database, "attachments", "path").await? {
        return Ok(());
    }

    if !sqlite_has_column(database, "attachments", "object_key").await? {
        execute_sqlite(
            database,
            "ALTER TABLE attachments ADD COLUMN object_key varchar NOT NULL DEFAULT ''",
        )
        .await?;
        execute_sqlite(database, "UPDATE attachments SET object_key = path").await?;
    }
    execute_sqlite(
        database,
        "UPDATE attachments SET object_key = 'attachments/' || storage_engine_id || '/' || path WHERE object_key = path AND path NOT LIKE 'attachments/%'",
    )
    .await?;
    if !sqlite_has_column(database, "attachments", "filename").await? {
        execute_sqlite(
            database,
            "ALTER TABLE attachments ADD COLUMN filename varchar NOT NULL DEFAULT ''",
        )
        .await?;
        execute_sqlite(database, "UPDATE attachments SET filename = path").await?;
    }
    if !sqlite_has_column(database, "attachments", "byte_size").await? {
        execute_sqlite(
            database,
            "ALTER TABLE attachments ADD COLUMN byte_size bigint NOT NULL DEFAULT 0",
        )
        .await?;
    }

    Ok(())
}

async fn sqlite_has_column(
    database: &DatabaseConnection,
    table: &str,
    column: &str,
) -> Result<bool> {
    let rows = database
        .query_all_raw(Statement::from_string(
            DbBackend::Sqlite,
            format!("PRAGMA table_info({table})"),
        ))
        .await?;

    Ok(rows.iter().any(|row| {
        row.try_get::<String>("", "name")
            .is_ok_and(|name| name == column)
    }))
}

async fn execute_sqlite(database: &DatabaseConnection, statement: &str) -> Result<()> {
    database
        .execute_raw(Statement::from_string(
            DbBackend::Sqlite,
            statement.to_owned(),
        ))
        .await?;
    Ok(())
}

pub async fn create_admin(database: &DatabaseConnection) {
    println!("Creating admin user...");

    let mut username = String::new();
    print!("Username: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut username).unwrap();
    let username = username.trim().to_string();

    let mut email = String::new();
    print!("Email: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut email).unwrap();
    let email = email.trim().to_string();

    let mut nickname = String::new();
    print!("Nickname: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut nickname).unwrap();
    let nickname = nickname.trim().to_string();

    let mut password = String::new();
    print!("Password: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut password).unwrap();
    let password = password.trim().to_string();

    if username.is_empty() || password.is_empty() {
        eprintln!("Username and password cannot be empty!");
        return;
    }

    let password_hash =
        bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("Failed to hash password");

    let user = user::ActiveModel {
        username: Set(username),
        email: Set(email),
        nickname: Set(nickname),
        password_hash: Set(password_hash),
        ..Default::default()
    };

    match user.insert(database).await {
        Ok(_) => println!("Admin user created successfully!"),
        Err(e) => eprintln!("Failed to create admin user: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::sync_entities;
    use crate::entity::{attachment, storage_engine};
    use sea_orm::{ConnectionTrait, Database, DbBackend, EntityTrait, Statement};

    #[tokio::test]
    async fn sync_entities_upgrades_legacy_sqlite_storage_tables() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        database
            .execute_raw(Statement::from_string(
                DbBackend::Sqlite,
                "CREATE TABLE storage_engines (id integer PRIMARY KEY, name varchar NOT NULL UNIQUE, comments varchar NOT NULL, type varchar NOT NULL, config varchar)".to_owned(),
            ))
            .await
            .unwrap();
        database
            .execute_raw(Statement::from_string(
                DbBackend::Sqlite,
                "INSERT INTO storage_engines (id, name, comments, type, config) VALUES (1, 'Legacy', 'Legacy engine', 'internal', NULL)".to_owned(),
            ))
            .await
            .unwrap();
        database
            .execute_raw(Statement::from_string(
                DbBackend::Sqlite,
                "CREATE TABLE attachments (id integer PRIMARY KEY, mime varchar NOT NULL, hash varchar NOT NULL UNIQUE, storage_engine_id integer NOT NULL, path varchar NOT NULL, created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP)".to_owned(),
            ))
            .await
            .unwrap();
        database
            .execute_raw(Statement::from_string(
                DbBackend::Sqlite,
                "INSERT INTO attachments (id, mime, hash, storage_engine_id, path) VALUES (1, 'text/plain', 'abc', 1, 'abc.txt')".to_owned(),
            ))
            .await
            .unwrap();

        sync_entities(&database).await.unwrap();

        let engine = storage_engine::Entity::find_by_id(1)
            .one(&database)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(engine.kind, "internal");
        assert!(engine.is_default);
        assert!(engine.enabled);

        let attachment = attachment::Entity::find_by_id(1)
            .one(&database)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(attachment.object_key, "attachments/1/abc.txt");
        assert_eq!(attachment.filename, "abc.txt");
        assert_eq!(attachment.byte_size, 0);
    }
}
