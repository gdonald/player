use std::path::PathBuf;

use clap::Parser;
use player_server::app::{self, AppState};
use player_server::cli::{self, Cli, Command, PasswordPrompt};
use player_server::{db, sources};
use tracing_subscriber::EnvFilter;

struct TerminalPrompt;

impl PasswordPrompt for TerminalPrompt {
    fn prompt(&mut self, label: &str) -> std::io::Result<String> {
        rpassword::prompt_password(label)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,lofty=error")),
        )
        .init();

    let cli = Cli::parse();
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/player_development".to_string());
    let pool = db::connect(&database_url).await?;

    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => serve(pool).await,
        command => {
            cli::run(
                command,
                &pool,
                &mut TerminalPrompt,
                &mut std::io::stdout(),
                bcrypt::DEFAULT_COST,
            )
            .await
        }
    }
}

async fn serve(pool: sqlx::PgPool) -> anyhow::Result<()> {
    db::migrate(&pool).await?;

    let reset = sources::reset_interrupted_scans(&pool).await?;
    if reset > 0 {
        tracing::warn!("{reset} interrupted scan(s) marked errored");
    }

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(3000);
    let web_root = PathBuf::from(
        std::env::var("WEB_ROOT").unwrap_or_else(|_| "crates/client/dist".to_string()),
    );

    let app = app::build(AppState::new(pool), Some(&web_root));

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!("listening on {}", listener.local_addr()?);

    axum::serve(
        listener,
        axum::ServiceExt::<axum::extract::Request>::into_make_service(app),
    )
    .with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.ok();
    })
    .await?;

    Ok(())
}
