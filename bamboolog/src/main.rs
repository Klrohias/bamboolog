use bamboolog::{config::ApplicationConfiguration, maintenance, web};
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use std::sync::Arc;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Parser)]
#[command(
    name = "bamboolog",
    about = "Run the Bamboolog server or maintenance commands"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Synchronize database entities and seed the default storage engine.
    SyncEntitiesEf,
    /// Interactively create an administrator account.
    CreateAdmin,
}

fn configure_tracing() {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn run_maintenance(command: Command, config: &ApplicationConfiguration) {
    let database = config
        .connect_database()
        .await
        .expect("Failed to connect to database");

    match command {
        Command::SyncEntitiesEf => maintenance::sync_entities(&database)
            .await
            .expect("Failed to sync entities"),
        Command::CreateAdmin => maintenance::create_admin(&database).await,
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    configure_tracing();

    let cli = Cli::parse();
    let config = Arc::new(ApplicationConfiguration::load().expect("Failed to load configuration"));

    match cli.command {
        Some(command) => run_maintenance(command, &config).await,
        None => web::run(config).await,
    }
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command};
    use clap::Parser;

    #[test]
    fn parses_server_mode_without_a_subcommand() {
        let cli = Cli::try_parse_from(["bamboolog"]).unwrap();

        assert!(cli.command.is_none());
    }

    #[test]
    fn parses_each_maintenance_subcommand() {
        let sync = Cli::try_parse_from(["bamboolog", "sync-entities-ef"]).unwrap();
        let create_admin = Cli::try_parse_from(["bamboolog", "create-admin"]).unwrap();

        assert!(matches!(sync.command, Some(Command::SyncEntitiesEf)));
        assert!(matches!(create_admin.command, Some(Command::CreateAdmin)));
    }

    #[test]
    fn rejects_unknown_arguments() {
        assert!(Cli::try_parse_from(["bamboolog", "unknown-command"]).is_err());
    }
}
