use clap::Parser;
use player_server::app::{self, Settings};
use player_server::cli::{self, Cli, Command, PasswordPrompt};
use player_server::db;

struct TerminalPrompt;

impl PasswordPrompt for TerminalPrompt {
    fn prompt(&mut self, label: &str) -> std::io::Result<String> {
        rpassword::prompt_password(label)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let settings = Settings::from_env(|name| std::env::var(name).ok());

    tracing_subscriber::fmt()
        .with_env_filter(app::log_filter(&settings.log_filter))
        .init();

    let cli = Cli::parse();
    let pool = db::connect(&settings.database_url).await?;

    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => {
            app::prepare(&pool).await?;

            let listener = tokio::net::TcpListener::bind(("0.0.0.0", settings.port)).await?;
            app::serve(listener, pool, &settings.web_root, async {
                tokio::signal::ctrl_c().await.ok();
            })
            .await
        }
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
