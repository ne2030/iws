use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands;

#[derive(Parser)]
#[command(name = "iws", about = "Multi-repo workspace manager — git worktree orchestration for multi-repo projects")]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize iws in the current directory (creates .workspaces/config.toml)
    Init,

    /// Create a new workspace with worktrees for specified repos
    New {
        /// Workspace name (e.g. auth-refactor)
        name: String,
        /// Comma-separated repo names
        #[arg(short, long, value_delimiter = ',')]
        repos: Vec<String>,
        /// Custom branch name (default: feature/{name})
        #[arg(short, long)]
        branch: Option<String>,
    },

    /// List all workspaces with status summary
    List,

    /// Show detailed status for a workspace
    Status {
        /// Workspace name
        name: String,
    },

    /// Open workspace in editor
    Open {
        /// Workspace name
        name: String,
        /// Editor to use (default: zed)
        #[arg(short, long, default_value = "zed")]
        editor: String,
    },

    /// Start a Claude Code session in the workspace
    Claude {
        /// Workspace name
        name: String,
    },

    /// Add a repo to an existing workspace
    AddRepo {
        /// Workspace name
        name: String,
        /// Repo name to add
        repo: String,
        /// Custom branch name
        #[arg(short, long)]
        branch: Option<String>,
    },

    /// Remove a repo from a workspace
    RemoveRepo {
        /// Workspace name
        name: String,
        /// Repo name to remove
        repo: String,
    },

    /// Create PRs for all repos in a workspace
    Pr {
        /// Workspace name
        name: String,
        /// PR title (default: workspace name)
        #[arg(short, long)]
        title: Option<String>,
        /// PR body/description
        #[arg(long)]
        body: Option<String>,
    },

    /// Clean up a workspace (remove worktrees and branches)
    Done {
        /// Workspace name
        name: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

impl Cli {
    pub fn run(self) -> Result<()> {
        match self.command {
            None => crate::tui::run(),
            Some(cmd) => match cmd {
                Command::Init => commands::init::run(),
                Command::New { name, repos, branch } => commands::new::run(&name, &repos, branch.as_deref()),
                Command::List => commands::list::run(),
                Command::Status { name } => commands::status::run(&name),
                Command::Open { name, editor } => commands::open::run(&name, &editor),
                Command::Claude { name } => commands::open::run_claude(&name),
                Command::AddRepo { name, repo, branch } => commands::repo::add(&name, &repo, branch.as_deref()),
                Command::RemoveRepo { name, repo } => commands::repo::remove(&name, &repo),
                Command::Pr { name, title, body } => commands::pr::run(&name, title.as_deref(), body.as_deref()),
                Command::Done { name, force } => commands::done::run(&name, force),
            },
        }
    }
}
