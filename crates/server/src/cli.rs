use std::io::Write;

use clap::{Parser, Subcommand};
use sqlx::PgPool;

use crate::users::{self, NewUser};
use crate::{db, seeds, sources};

#[derive(Debug, Parser)]
#[command(name = "player", about = "Web-based MP3 player")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the web server (the default).
    Serve,
    /// Apply database migrations.
    Migrate,
    /// Create the Unknown artist and album, the Recently Played playlist, and user gd.
    Seed,
    /// Manage users.
    User {
        #[command(subcommand)]
        command: UserCommand,
    },
    /// Manage music sources.
    Source {
        #[command(subcommand)]
        command: SourceCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum UserCommand {
    /// Add a user, prompting for the password twice.
    Add { username: String },
}

#[derive(Debug, Subcommand)]
pub enum SourceCommand {
    /// Add a directory to scan for MP3s.
    Add { path: String },
}

pub trait PasswordPrompt {
    fn prompt(&mut self, label: &str) -> std::io::Result<String>;
}

pub async fn run(
    command: Command,
    pool: &PgPool,
    prompt: &mut dyn PasswordPrompt,
    output: &mut dyn Write,
    bcrypt_cost: u32,
) -> anyhow::Result<()> {
    match command {
        Command::Serve => anyhow::bail!("serve is handled by the binary entry point"),
        Command::Migrate => {
            db::migrate(pool).await?;
            writeln!(output, "Migrations applied")?;
        }
        Command::Seed => {
            seeds::run(pool, bcrypt_cost).await?;
            writeln!(output, "Seeded")?;
        }
        Command::User {
            command: UserCommand::Add { username },
        } => {
            let password = prompt.prompt("Password: ")?;
            let password_confirmation = prompt.prompt("Confirm password: ")?;

            users::create(
                pool,
                NewUser {
                    username: &username,
                    password: &password,
                    password_confirmation: &password_confirmation,
                },
                bcrypt_cost,
            )
            .await?;

            writeln!(output, "Created user {}", username.trim().to_lowercase())?;
        }
        Command::Source {
            command: SourceCommand::Add { path },
        } => {
            let source = sources::create(pool, &path).await?;
            writeln!(output, "Created source {} at {}", source.id, source.path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    struct ScriptedPrompt {
        answers: VecDeque<&'static str>,
        labels: Vec<String>,
    }

    impl ScriptedPrompt {
        fn new(answers: &[&'static str]) -> Self {
            ScriptedPrompt {
                answers: answers.iter().copied().collect(),
                labels: Vec::new(),
            }
        }
    }

    impl PasswordPrompt for ScriptedPrompt {
        fn prompt(&mut self, label: &str) -> std::io::Result<String> {
            self.labels.push(label.to_string());

            self.answers
                .pop_front()
                .map(String::from)
                .ok_or_else(|| std::io::Error::other("no more answers"))
        }
    }

    async fn run_command(
        pool: &PgPool,
        command: Command,
        answers: &[&'static str],
    ) -> (anyhow::Result<()>, String) {
        let mut prompt = ScriptedPrompt::new(answers);
        let mut output = Vec::new();

        let result = run(command, pool, &mut prompt, &mut output, 4).await;

        (result, String::from_utf8(output).unwrap())
    }

    #[test]
    fn no_subcommand_parses_as_none() {
        assert!(Cli::parse_from(["player"]).command.is_none());
    }

    #[test]
    fn subcommands_parse() {
        assert!(matches!(
            Cli::parse_from(["player", "user", "add", "gd"]).command,
            Some(Command::User {
                command: UserCommand::Add { .. }
            })
        ));
    }

    #[sqlx::test]
    async fn serve_is_not_run_here(pool: PgPool) {
        let (result, _) = run_command(&pool, Command::Serve, &[]).await;

        assert!(result.is_err());
    }

    #[sqlx::test]
    async fn migrate_reports_success(pool: PgPool) {
        let (result, output) = run_command(&pool, Command::Migrate, &[]).await;

        result.unwrap();
        assert_eq!(output, "Migrations applied\n");
    }

    #[sqlx::test]
    async fn seed_reports_success(pool: PgPool) {
        let (result, output) = run_command(&pool, Command::Seed, &[]).await;

        result.unwrap();
        assert_eq!(output, "Seeded\n");
    }

    #[sqlx::test]
    async fn user_add_prompts_twice_and_creates_the_user(pool: PgPool) {
        let mut prompt = ScriptedPrompt::new(&["secret", "secret"]);
        let mut output = Vec::new();

        run(
            Command::User {
                command: UserCommand::Add {
                    username: "Greg".to_string(),
                },
            },
            &pool,
            &mut prompt,
            &mut output,
            4,
        )
        .await
        .unwrap();

        assert_eq!(prompt.labels, vec!["Password: ", "Confirm password: "]);
        assert_eq!(String::from_utf8(output).unwrap(), "Created user greg\n");
        assert!(
            users::authenticate(&pool, "greg", "secret")
                .await
                .unwrap()
                .is_some()
        );
    }

    #[sqlx::test]
    async fn user_add_fails_when_the_confirmation_differs(pool: PgPool) {
        let command = Command::User {
            command: UserCommand::Add {
                username: "gd".to_string(),
            },
        };

        let (result, _) = run_command(&pool, command, &["one", "two"]).await;

        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed to create user: password_confirmation doesn't match Password"
        );
    }

    #[sqlx::test]
    async fn user_add_fails_when_the_prompt_fails(pool: PgPool) {
        let command = Command::User {
            command: UserCommand::Add {
                username: "gd".to_string(),
            },
        };

        let (result, _) = run_command(&pool, command, &["only one"]).await;

        assert_eq!(result.unwrap_err().to_string(), "no more answers");
    }

    #[sqlx::test]
    async fn source_add_creates_the_source(pool: PgPool) {
        let command = Command::Source {
            command: SourceCommand::Add {
                path: "/music".to_string(),
            },
        };

        let (result, output) = run_command(&pool, command, &[]).await;

        result.unwrap();
        assert!(output.starts_with("Created source ") && output.ends_with(" at /music\n"));
    }
}
