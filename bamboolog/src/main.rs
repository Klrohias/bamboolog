use bamboolog::{
    self,
    config::{ApplicationConfiguration, config_entries},
    router::get_routes,
    service::{
        jwt::{JwtService, JwtServiceSettings, JwtServiceState},
        reloader::ServiceReloader,
        theme::{ThemeService, ThemeServiceSettings, ThemeServiceState},
    },
};

use axum::{Extension, Router};
use dotenvy::dotenv;
use sea_orm::{Database, DatabaseConnection};
use std::{
    env::{Args, args},
    net::SocketAddr,
    sync::Arc,
};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tracing::instrument;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn configure_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn configure_database(config: &ApplicationConfiguration) -> DatabaseConnection {
    Database::connect(&config.database)
        .await
        .expect("Failed to connect to database")
}

#[instrument(skip_all)]
async fn configure_jwt_service(database: &DatabaseConnection) -> JwtService {
    JwtService::new(JwtServiceState::from(
        config_entries::JWT_CONFIG
            .get::<JwtServiceSettings>(database)
            .await
            .expect("Failed to config jwt service")
            .unwrap_or_default(),
    ))
}

#[instrument(skip_all)]
async fn configure_theme_service(database: &DatabaseConnection) -> ThemeService {
    let settings = config_entries::THEME_SERVICE_CONFIG
        .get::<ThemeServiceSettings>(&database)
        .await
        .expect("Failed to load config for theme service")
        .expect("No config for theme service");

    let mut state = ThemeServiceState::default();
    state
        .load(&settings)
        .expect("Failed to load theme service state");

    ThemeService::new(state)
}

async fn build_app(config: Arc<ApplicationConfiguration>) -> Router {
    // Configure services
    let database = configure_database(&config).await;
    let jwt_service = configure_jwt_service(&database).await;
    let theme_service = configure_theme_service(&database).await;
    let service_reloader =
        ServiceReloader::new(database.clone(), jwt_service.clone(), theme_service.clone());

    // Create routes
    get_routes().layer(
        ServiceBuilder::new()
            .layer(Extension(config.clone()))
            .layer(Extension(database))
            .layer(Extension(jwt_service))
            .layer(Extension(theme_service))
            .layer(Extension(service_reloader)),
    )
}

async fn action_dispatch(mut args: Args, config: &Arc<ApplicationConfiguration>) -> bool {
    if args.any(|x| x == "sync-entities-ef") {
        action_sync_entities(config).await;
        return true;
    }

    false
}

async fn action_sync_entities(config: &Arc<ApplicationConfiguration>) {
    tracing::info!("Sync entities (Entity first)");
    let db = configure_database(&config).await;

    db.get_schema_registry("bamboolog::entity::*")
        .sync(&db)
        .await
        .expect("Failed to sync schemas");
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    configure_tracing();

    let config = Arc::new(ApplicationConfiguration::load().expect("Failed to load configuration"));

    if action_dispatch(args(), &config).await {
        return;
    }

    let app = build_app(config.clone()).await;

    let addr: SocketAddr = config
        .listen_addr
        .as_str()
        .parse()
        .expect("Invalid listen_addr");
    println!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
