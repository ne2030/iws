use anyhow::{Result, bail};
use colored::Colorize;

use crate::config::Config;
use crate::workspace::WorkspaceConfig;
use crate::git;

pub fn add(ws_name: &str, repo_name: &str, branch: Option<&str>) -> Result<()> {
    let root = Config::find_root()?;
    let config = Config::load(&root)?;
    let mut ws = WorkspaceConfig::load(&root, ws_name)?;

    // Validate repo exists
    config.get_repo(repo_name)?;

    if ws.repos.contains_key(repo_name) {
        bail!("Repo '{}' is already in workspace '{}'", repo_name, ws_name);
    }

    // Determine branch — use existing workspace branch pattern or custom
    let branch_name = branch
        .map(|b| b.to_string())
        .unwrap_or_else(|| {
            ws.repos.values().next()
                .map(|r| r.branch.clone())
                .unwrap_or_else(|| format!("feature/{}", ws_name))
        });

    let repo_path = config.repo_abs_path(repo_name)?;
    let ws_dir = WorkspaceConfig::workspace_dir(&root, ws_name);
    let target = ws_dir.join(repo_name);

    git::worktree_add(&repo_path, &target, &branch_name)?;
    ws.add_repo(repo_name, &branch_name);
    ws.save(&root)?;

    // Symlink shared files
    let repo_cfg = config.get_repo(repo_name)?;
    if !repo_cfg.shared_files.is_empty() {
        git::copy_shared_files(&repo_path, &target, &repo_cfg.shared_files);
    }

    println!("{} Added {} to workspace '{}' (branch: {})",
        "✓".green(), repo_name.bold(), ws_name, branch_name.cyan());
    Ok(())
}

pub fn remove(ws_name: &str, repo_name: &str) -> Result<()> {
    let root = Config::find_root()?;
    let config = Config::load(&root)?;
    let mut ws = WorkspaceConfig::load(&root, ws_name)?;

    if !ws.repos.contains_key(repo_name) {
        bail!("Repo '{}' is not in workspace '{}'", repo_name, ws_name);
    }

    let ws_dir = WorkspaceConfig::workspace_dir(&root, ws_name);
    let target = ws_dir.join(repo_name);

    if target.exists() {
        let repo_path = config.repo_abs_path(repo_name)?;
        git::worktree_remove(&repo_path, &target)?;
    }

    ws.remove_repo(repo_name);
    ws.save(&root)?;

    println!("{} Removed {} from workspace '{}'", "✓".green(), repo_name.bold(), ws_name);
    Ok(())
}
