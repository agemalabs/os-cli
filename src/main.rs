//! OS CLI — Agema Labs operating system terminal interface.
//!
//! Usage:
//!   os                      Launch TUI at dashboard
//!   os @<project>           Launch TUI focused on project
//!   os push <path> @<slug>  Push file to project
//!   os pull @<slug>         Pull latest from project
//!   os search "query"       Semantic search
//!   os loop                 Daily briefing
//!   os task "title" @<slug> Create task

mod api_client;
mod commands;
mod config;
mod tui;

use clap::{Parser, Subcommand};

/// OS — Agema Labs operating system
#[derive(Parser)]
#[command(name = "os", version, about = "Agema Labs OS — terminal interface")]
struct Cli {
    /// Launch TUI focused on a project (e.g. @brownells)
    #[arg(value_name = "PROJECT")]
    project: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate via OAuth (GitHub or Google)
    Login {
        /// Use Google instead of GitHub
        #[arg(long)]
        google: bool,
    },
    /// Push a file or directory to a project
    Push {
        /// Path to file or directory
        path: String,
        /// Target project (e.g. @brownells)
        #[arg(value_name = "PROJECT")]
        project: String,
        /// File category
        #[arg(long)]
        category: Option<String>,
    },
    /// Pull latest files from a project
    Pull {
        /// Project slug (e.g. @brownells)
        #[arg(value_name = "PROJECT")]
        project: String,
    },
    /// Watch and sync files on change
    Sync {
        /// Enable watch mode
        #[arg(long)]
        watch: bool,
        /// Project slug
        #[arg(value_name = "PROJECT")]
        project: String,
    },
    /// Create a task in a project
    Task {
        /// Task title
        title: String,
        /// Project slug (e.g. @brownells)
        #[arg(value_name = "PROJECT")]
        project: String,
        /// Assign to team member
        #[arg(long)]
        assign: Option<String>,
        /// Due date
        #[arg(long)]
        due: Option<String>,
    },
    /// List tasks for a project
    Tasks {
        /// Project slug
        #[arg(value_name = "PROJECT")]
        project: String,
    },
    /// Semantic search across projects
    Search {
        /// Search query
        query: String,
        /// Limit to project
        #[arg(value_name = "PROJECT")]
        project: Option<String>,
    },
    /// Daily briefing — what matters today
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show project meta
    Meta {
        /// Project slug
        #[arg(value_name = "PROJECT")]
        project: String,
    },
    /// Show or regenerate MCP token
    Token {
        /// Regenerate token
        #[arg(long)]
        regenerate: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Load config (creates default if first run)
    let mut cfg = config::load_or_init()?;

    // Auto-trigger login on first run if no token
    if cfg.token.is_empty() && !matches!(cli.command, Some(Commands::Login { .. })) {
        println!("First run — authentication required.\n");
        println!("  1) Google");
        println!("  2) GitHub\n");
        // Default to Google since it's available
        commands::auth::login(&cfg.api_url, commands::auth::Provider::Google).await?;
        cfg = config::load_or_init()?;
    }

    let client = api_client::ApiClient::new(&cfg.api_url, &cfg.token);

    match cli.command {
        Some(Commands::Login { google }) => {
            let provider = if google {
                commands::auth::Provider::Google
            } else {
                commands::auth::Provider::GitHub
            };
            commands::auth::login(&cfg.api_url, provider).await?;
        }
        Some(Commands::Push {
            path,
            project,
            category,
        }) => {
            commands::push::run(&client, &path, &project, category.as_deref()).await?;
        }
        Some(Commands::Pull { project }) => {
            commands::pull::run(&client, &project).await?;
        }
        Some(Commands::Sync { watch, project }) => {
            commands::sync::run(&client, &project, watch).await?;
        }
        Some(Commands::Search { query, project }) => {
            commands::search::run(&client, &query, project.as_deref()).await?;
        }
        Some(Commands::Status { json }) => {
            commands::loop_cmd::run(&client, json).await?;
        }
        Some(Commands::Meta { project }) => {
            commands::meta::run(&client, &project).await?;
        }
        Some(Commands::Task {
            title,
            project,
            assign,
            due,
        }) => {
            commands::task::create(&client, &title, &project, assign.as_deref(), due.as_deref())
                .await?;
        }
        Some(Commands::Tasks { project }) => {
            commands::task::list(&client, &project).await?;
        }
        Some(Commands::Token { regenerate }) => {
            commands::token::run(&client, regenerate).await?;
        }
        None => {
            tui::run(client, cli.project).await?;
        }
    }

    Ok(())
}
