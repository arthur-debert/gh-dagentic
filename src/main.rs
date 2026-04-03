mod commands;
mod config;
mod context;
mod fs;
mod gh;
mod git;
mod labels;
#[allow(dead_code)] // Used by upcoming show/stats commands
mod metadata;
mod pipeline;
mod templates;

use clap::{Parser, Subcommand};
use clapfig::{Boundary, Clapfig, SearchPath};
use config::DagenticConfig;
use context::Context;
use fs::RealFs;
use gh::GhCli;
use git::GitCli;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(
    name = "gh-dagentic",
    about = "Agentic development workflow orchestration",
    version = VERSION,
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Set up Dagentic in the current repository
    Init,
    /// Re-sync workflow files, issue templates, and labels
    Update,
    /// List tasks grouped by pipeline stage
    List {
        /// Filter by stage: planning, planned, approved, coding, review, done, abandoned
        #[arg(long)]
        stage: Option<String>,
    },
    /// Show the current state of the pipeline (alias for list)
    Status,
}

fn main() {
    let cli = Cli::parse();
    let config: DagenticConfig = Clapfig::builder()
        .app_name("dagentic")
        .search_paths(vec![SearchPath::Ancestors(Boundary::Marker(".git"))])
        .load()
        .unwrap_or_else(|e| {
            eprintln!("Warning: could not load config: {e}");
            DagenticConfig::default()
        });
    let fs = RealFs;
    let host = GhCli;
    let repo = GitCli;
    let ctx = Context {
        config: &config,
        fs: &fs,
        host: &host,
        repo: &repo,
    };

    let result = match cli.command {
        Some(Commands::Init) => commands::init::run(&ctx),
        Some(Commands::Update) => commands::update::run(&ctx),
        Some(Commands::List { ref stage }) => commands::list::run(&ctx, stage.as_deref()),
        Some(Commands::Status) => commands::list::run(&ctx, None),
        None => {
            Cli::parse_from(["gh-dagentic", "--help"]);
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
