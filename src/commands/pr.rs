use anyhow::{Result, bail};
use colored::Colorize;

use crate::config::Config;
use crate::workspace::WorkspaceConfig;
use crate::git;

pub fn run(name: &str, custom_title: Option<&str>, custom_body: Option<&str>) -> Result<()> {
    if !git::has_gh() {
        bail!("gh CLI is not installed. Install it with: brew install gh");
    }

    let root = Config::find_root()?;
    let ws = WorkspaceConfig::load(&root, name)?;
    let ws_dir = WorkspaceConfig::workspace_dir(&root, name);

    let mut pr_urls: Vec<(String, String)> = Vec::new();

    for (repo_name, repo_info) in &ws.repos {
        let wt_path = ws_dir.join(repo_name);
        if !wt_path.exists() {
            eprintln!("  {} {} — worktree missing, skipping", "⚠".yellow(), repo_name);
            continue;
        }

        println!("  Pushing and creating PR for {}...", repo_name.bold());

        // Push first
        if let Err(e) = git::push(&wt_path, &repo_info.branch) {
            eprintln!("    {} Push failed: {}", "✗".red(), e);
            continue;
        }

        // Build PR body with cross-references to other repos
        let other_repos: Vec<&str> = ws.repos.keys()
            .filter(|r| r.as_str() != repo_name)
            .map(|r| r.as_str())
            .collect();

        let cross_ref = if other_repos.is_empty() {
            String::new()
        } else {
            let refs: Vec<String> = other_repos.iter()
                .map(|r| format!("- `{}` (branch: `{}`)", r, repo_info.branch))
                .collect();
            format!("\n\n## Related repos\n{}", refs.join("\n"))
        };

        // Add links to already-created PRs
        let pr_links = if pr_urls.is_empty() {
            String::new()
        } else {
            let links: Vec<String> = pr_urls.iter()
                .map(|(r, url)| format!("- {}: {}", r, url))
                .collect();
            format!("\n\n## Related PRs\n{}", links.join("\n"))
        };

        let title = match custom_title {
            Some(t) => format!("{} ({})", t, repo_name),
            None => format!("{} ({})", name, repo_name),
        };
        let body = match custom_body {
            Some(b) => format!("{}{}{}", b, cross_ref, pr_links),
            None => format!("Workspace: `{}`\nBranch: `{}`{}{}", name, repo_info.branch, cross_ref, pr_links),
        };

        match git::create_pr(&wt_path, &title, &body) {
            Ok(url) => {
                println!("    {} {} → {}", "✓".green(), repo_name, url);
                pr_urls.push((repo_name.clone(), url));
            }
            Err(e) => {
                eprintln!("    {} PR creation failed: {}", "✗".red(), e);
            }
        }
    }

    if pr_urls.is_empty() {
        println!("\n{} No PRs were created.", "⚠".yellow());
    } else {
        println!("\n{} Created {} PR(s):", "✓".green().bold(), pr_urls.len());
        for (repo, url) in &pr_urls {
            println!("  {} → {}", repo.bold(), url);
        }
    }

    Ok(())
}
