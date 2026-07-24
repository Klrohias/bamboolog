use crate::entity::user;
use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::io::{self, Write};

pub async fn sync_entities(database: &DatabaseConnection) -> Result<()> {
    tracing::info!("Sync entities (Entity first)");

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
