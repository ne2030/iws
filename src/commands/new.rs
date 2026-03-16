use anyhow::{Result, bail};
use colored::Colorize;

use crate::config::Config;
use crate::workspace::WorkspaceConfig;
use crate::git;
use crate::spec;
use crate::claude;

pub fn run(name: &str, repos: &[String], branch: Option<&str>) -> Result<()> {
    let root = Config::find_root()?;
    let config = Config::load(&root)?;

    // Validate all repos exist in config
    for repo in repos {
        config.get_repo(repo)?;
    }

    let ws_dir = WorkspaceConfig::workspace_dir(&root, name);
    if ws_dir.exists() {
        bail!("Workspace '{}' already exists at {}", name, ws_dir.display());
    }

    let branch_name = branch
        .map(|b| b.to_string())
        .unwrap_or_else(|| format!("feature/{}", name));

    // Create workspace directory
    std::fs::create_dir_all(&ws_dir)?;

    // Create worktrees
    let mut ws_config = WorkspaceConfig::new(name);

    for repo_name in repos {
        let repo_path = config.repo_abs_path(repo_name)?;
        let target = ws_dir.join(repo_name);

        println!("  Creating worktree for {}...", repo_name.bold());
        match git::worktree_add(&repo_path, &target, &branch_name) {
            Ok(()) => {
                ws_config.add_repo(repo_name, &branch_name);
                println!("    {} {} → {}", "✓".green(), repo_name, branch_name);
                // Symlink shared files (.env, keyfiles, etc.)
                let repo_cfg = config.get_repo(repo_name)?;
                if !repo_cfg.shared_files.is_empty() {
                    git::copy_shared_files(&repo_path, &target, &repo_cfg.shared_files);
                }
            }
            Err(e) => {
                eprintln!("    {} {} — {}", "✗".red(), repo_name, e);
                // Clean up already-created worktrees
                for (created_repo, _) in &ws_config.repos {
                    let created_target = ws_dir.join(created_repo);
                    let created_repo_path = config.repo_abs_path(created_repo).ok();
                    if let Some(rp) = created_repo_path {
                        let _ = git::worktree_remove(&rp, &created_target);
                    }
                }
                let _ = std::fs::remove_dir_all(&ws_dir);
                bail!("Failed to create worktree for '{}': {}", repo_name, e);
            }
        }
    }

    // Save workspace config
    ws_config.save(&root)?;

    // Generate SPEC.md and .claude/CLAUDE.md
    let repo_names: Vec<String> = repos.to_vec();
    spec::generate(&ws_dir, name, &repo_names)?;
    claude::generate(&ws_dir, name)?;

    println!("\n{} Workspace '{}' created at {}", "✓".green().bold(), name.bold(), ws_dir.display());
    println!("  Branch: {}", branch_name.cyan());
    println!("  Repos:  {}", repos.join(", "));
    println!("\n  Next steps:");
    println!("    Edit {}SPEC.md with requirements", format!(".workspaces/{}/", name));
    println!("    iws open {}", name);
    println!("    iws claude {}", name);

    Ok(())
}
