use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub project: ProjectConfig,
    pub repos: BTreeMap<String, RepoConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub root: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoConfig {
    pub path: String,
    pub default_branch: String,
    /// Files to copy from main repo into worktrees (e.g. .env, keyfiles)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_files: Vec<String>,
}

impl Config {
    /// Find the project root by walking up from cwd looking for .workspaces/config.toml
    pub fn find_root() -> Result<PathBuf> {
        let cwd = std::env::current_dir()?;
        let mut dir = cwd.as_path();
        loop {
            let config_path = dir.join(".workspaces").join("config.toml");
            if config_path.exists() {
                return Ok(dir.to_path_buf());
            }
            dir = dir.parent().context("Could not find iws root (no .workspaces/config.toml found). Run `iws init` first.")?;
        }
    }

    pub fn config_path(root: &Path) -> PathBuf {
        root.join(".workspaces").join("config.toml")
    }

    pub fn workspaces_dir(root: &Path) -> PathBuf {
        root.join(".workspaces")
    }

    pub fn load(root: &Path) -> Result<Self> {
        let path = Self::config_path(root);
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        toml::from_str(&content).context("Failed to parse config.toml")
    }

    pub fn save(&self, root: &Path) -> Result<()> {
        let path = Self::config_path(root);
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Auto-detect repos by scanning for git directories in the root
    pub fn detect_repos(root: &Path) -> BTreeMap<String, RepoConfig> {
        let mut repos = BTreeMap::new();

        let entries = match std::fs::read_dir(root) {
            Ok(entries) => entries,
            Err(_) => return repos,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name = match entry.file_name().to_str() {
                Some(n) => n.to_string(),
                None => continue,
            };

            // Skip hidden dirs and .workspaces
            if name.starts_with('.') {
                continue;
            }

            // Check if it's a git repo (regular .git dir or worktree .git file)
            if path.join(".git").exists() {
                let default_branch = detect_default_branch(&path).unwrap_or_else(|| "main".to_string());
                repos.insert(name.clone(), RepoConfig {
                    path: name,
                    default_branch,
                    shared_files: Vec::new(),
                });
            }
        }
        repos
    }

    pub fn get_repo(&self, name: &str) -> Result<&RepoConfig> {
        self.repos.get(name).with_context(|| {
            let available: Vec<&str> = self.repos.keys().map(|s| s.as_str()).collect();
            format!("Unknown repo '{}'. Available: {}", name, available.join(", "))
        })
    }

    pub fn repo_abs_path(&self, repo_name: &str) -> Result<PathBuf> {
        let repo = self.get_repo(repo_name)?;
        let root = PathBuf::from(&self.project.root);
        let path = root.join(&repo.path);
        if !path.exists() {
            bail!("Repo path does not exist: {}", path.display());
        }
        Ok(path)
    }
}

fn detect_default_branch(repo_path: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD", "--short"])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // Strip "origin/" prefix
        Some(branch.strip_prefix("origin/").unwrap_or(&branch).to_string())
    } else {
        None
    }
}
