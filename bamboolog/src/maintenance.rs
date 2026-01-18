use crate::config::ApplicationConfiguration;
use crate::entity::user;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Database, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use std::env::Args;
use std::io::{self, Write};
use std::sync::Arc;

async fn configure_database(config: &ApplicationConfiguration) -> DatabaseConnection {
    Database::connect(&config.database)
        .await
        .expect("Failed to connect to database")
}

pub async fn action_dispatch(args: Args, config: &Arc<ApplicationConfiguration>) -> bool {
    let args_vec: Vec<String> = args.collect();

    if args_vec.iter().any(|x| x == "sync-entities-ef") {
        action_sync_entities(config).await;
        return true;
    }

    if args_vec.iter().any(|x| x == "create-admin") {
        action_create_admin(config).await;
        return true;
    }

    false
}

pub async fn action_sync_entities(config: &Arc<ApplicationConfiguration>) {
    tracing::info!("Sync entities (Entity first)");
    let db = configure_database(config).await;

    db.get_schema_registry("bamboolog::entity::*")
        .sync(&db)
        .await
        .expect("Failed to sync schemas");

    // Seed default storage engine
    use crate::entity::storage_engine;
    let internal_exists = storage_engine::Entity::find()
        .filter(storage_engine::Column::Type.eq("internal"))
        .one(&db)
        .await
        .expect("Failed to check storage engine");

    if internal_exists.is_none() {
        tracing::info!("Seeding default internal storage engine");
        let active_model = storage_engine::ActiveModel {
            name: Set("Local Storage".to_string()),
            comments: Set("Default local storage".to_string()),
            r#type: Set("internal".to_string()),
            config: Set(None),
            ..Default::default()
        };
        active_model
            .insert(&db)
            .await
            .expect("Failed to insert default storage engine");
    }
}

async fn action_create_admin(config: &Arc<ApplicationConfiguration>) {
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

    let db = configure_database(config).await;

    let user = user::ActiveModel {
        username: Set(username),
        email: Set(email),
        nickname: Set(nickname),
        password_hash: Set(password_hash),
        ..Default::default()
    };

    match user.insert(&db).await {
        Ok(_) => println!("Admin user created successfully!"),
        Err(e) => eprintln!("Failed to create admin user: {}", e),
    }
}
