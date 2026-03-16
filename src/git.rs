use std::path::Path;
use std::process::Command;
use anyhow::{Context, Result, bail};
use colored::Colorize;

/// Copy shared files (e.g. .env, keyfiles) from source repo into worktree.
/// Files are copied (not symlinked) so each worktree can have independent config.
pub fn copy_shared_files(repo_path: &Path, worktree_path: &Path, shared_files: &[String]) {
    for file in shared_files {
        let source = repo_path.join(file);
        let target = worktree_path.join(file);

        if !source.exists() {
            eprintln!("    {} shared file not found: {}", "⚠".yellow(), source.display());
            continue;
        }

        // Create parent dirs in worktree if needed
        if let Some(parent) = target.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match std::fs::copy(&source, &target) {
            Ok(_) => println!("    {} {} (copied)", "→".cyan(), file),
            Err(e) => eprintln!("    {} copy failed for {}: {}", "✗".red(), file, e),
        }
    }
}

/// Add a git worktree from a source repo to a target path with a new branch
pub fn worktree_add(repo_path: &Path, target_path: &Path, branch: &str) -> Result<()> {
    // Check if branch already exists
    let branch_exists = Command::new("git")
        .args(["rev-parse", "--verify", branch])
        .current_dir(repo_path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let output = if branch_exists {
        Command::new("git")
            .args(["worktree", "add", &target_path.to_string_lossy().to_string(), branch])
            .current_dir(repo_path)
            .output()
            .context("Failed to run git worktree add")?
    } else {
        Command::new("git")
            .args([
                "worktree", "add",
                &target_path.to_string_lossy().to_string(),
                "-b", branch,
            ])
            .current_dir(repo_path)
            .output()
            .context("Failed to run git worktree add")?
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git worktree add failed: {}", stderr.trim());
    }
    Ok(())
}

/// Remove a git worktree
pub fn worktree_remove(repo_path: &Path, target_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["worktree", "remove", &target_path.to_string_lossy().to_string()])
        .current_dir(repo_path)
        .output()
        .context("Failed to run git worktree remove")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Try force remove if normal remove fails
        let output2 = Command::new("git")
            .args(["worktree", "remove", "--force", &target_path.to_string_lossy().to_string()])
            .current_dir(repo_path)
            .output()
            .context("Failed to run git worktree remove --force")?;

        if !output2.status.success() {
            let stderr2 = String::from_utf8_lossy(&output2.stderr);
            bail!("git worktree remove failed: {} / {}", stderr.trim(), stderr2.trim());
        }
    }
    Ok(())
}

/// Delete a local branch
pub fn delete_branch(repo_path: &Path, branch: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["branch", "-d", branch])
        .current_dir(repo_path)
        .output()
        .context("Failed to delete branch")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git branch -d failed for '{}': {}", branch, stderr.trim());
    }
    Ok(())
}

/// Get the number of commits ahead of the default branch
pub fn commits_ahead(worktree_path: &Path, default_branch: &str) -> Result<usize> {
    let range = format!("{}..HEAD", default_branch);
    let output = Command::new("git")
        .args(["rev-list", "--count", &range])
        .current_dir(worktree_path)
        .output()
        .context("Failed to count commits ahead")?;

    if output.status.success() {
        let count: usize = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .unwrap_or(0);
        Ok(count)
    } else {
        Ok(0)
    }
}

/// Get the number of dirty (modified/untracked) files
pub fn dirty_count(worktree_path: &Path) -> Result<usize> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(worktree_path)
        .output()
        .context("Failed to get git status")?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let count = stdout.lines().filter(|l| !l.is_empty()).count();
        Ok(count)
    } else {
        Ok(0)
    }
}

/// Get the current branch name
pub fn current_branch(worktree_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(worktree_path)
        .output()
        .context("Failed to get current branch")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Ok("(detached)".to_string())
    }
}

/// Push the current branch to origin
pub fn push(worktree_path: &Path, branch: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["push", "-u", "origin", branch])
        .current_dir(worktree_path)
        .output()
        .context("Failed to push")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git push failed: {}", stderr.trim());
    }
    Ok(())
}

/// Check if gh CLI is available
pub fn has_gh() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Create a PR using gh CLI
pub fn create_pr(worktree_path: &Path, title: &str, body: &str) -> Result<String> {
    let output = Command::new("gh")
        .args(["pr", "create", "--title", title, "--body", body])
        .current_dir(worktree_path)
        .output()
        .context("Failed to create PR with gh")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh pr create failed: {}", stderr.trim());
    }
}
