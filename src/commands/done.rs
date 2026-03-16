use std::io::{self, Write};
use anyhow::Result;
use colored::Colorize;

use crate::config::Config;
use crate::workspace::WorkspaceConfig;
use crate::git;

pub fn run(name: &str, force: bool) -> Result<()> {
    let root = Config::find_root()?;
    let config = Config::load(&root)?;
    let ws = WorkspaceConfig::load(&root, name)?;
    let ws_dir = WorkspaceConfig::workspace_dir(&root, name);

    // Check for dirty worktrees
    if !force {
        let mut has_dirty = false;
        for repo_name in ws.repos.keys() {
            let wt_path = ws_dir.join(repo_name);
            if wt_path.exists() {
                let dirty = git::dirty_count(&wt_path).unwrap_or(0);
                if dirty > 0 {
                    eprintln!("  {} {} has {} uncommitted change(s)", "⚠".yellow(), repo_name, dirty);
                    has_dirty = true;
                }
            }
        }

        if has_dirty {
            print!("\nWorkspace has uncommitted changes. Continue? [y/N] ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Aborted.");
                return Ok(());
            }
        }
    }

    // Remove worktrees
    for (repo_name, repo_info) in &ws.repos {
        let wt_path = ws_dir.join(repo_name);
        if !wt_path.exists() {
            continue;
        }

        let repo_path = match config.repo_abs_path(repo_name) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("  {} {} — {}", "⚠".yellow(), repo_name, e);
                continue;
            }
        };

        print!("  Removing worktree {}...", repo_name.bold());
        match git::worktree_remove(&repo_path, &wt_path) {
            Ok(()) => {
                println!(" {}", "✓".green());
                // Try to delete the branch
                match git::delete_branch(&repo_path, &repo_info.branch) {
                    Ok(()) => println!("    Deleted branch: {}", repo_info.branch.dimmed()),
                    Err(_) => println!("    Branch '{}' kept (may have unmerged changes)", repo_info.branch.dimmed()),
                }
            }
            Err(e) => {
                println!(" {}", "✗".red());
                eprintln!("    {}", e);
            }
        }
    }

    // Remove workspace directory
    if ws_dir.exists() {
        std::fs::remove_dir_all(&ws_dir)?;
    }

    println!("\n{} Workspace '{}' cleaned up.", "✓".green().bold(), name.bold());

    Ok(())
}
