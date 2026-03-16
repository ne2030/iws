use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub workspace: WorkspaceMeta,
    pub repos: BTreeMap<String, WorkspaceRepo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceMeta {
    pub name: String,
    pub created: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceRepo {
    pub branch: String,
}

impl WorkspaceConfig {
    pub fn new(name: &str) -> Self {
        let created = chrono::Local::now().format("%Y-%m-%d").to_string();
        Self {
            workspace: WorkspaceMeta {
                name: name.to_string(),
                created,
            },
            repos: BTreeMap::new(),
        }
    }

    pub fn add_repo(&mut self, repo_name: &str, branch: &str) {
        self.repos.insert(repo_name.to_string(), WorkspaceRepo {
            branch: branch.to_string(),
        });
    }

    pub fn remove_repo(&mut self, repo_name: &str) -> bool {
        self.repos.remove(repo_name).is_some()
    }

    pub fn workspace_dir(root: &Path, name: &str) -> PathBuf {
        root.join(".workspaces").join(name)
    }

    pub fn config_path(root: &Path, name: &str) -> PathBuf {
        Self::workspace_dir(root, name).join(".workspace.toml")
    }

    pub fn load(root: &Path, name: &str) -> Result<Self> {
        let path = Self::config_path(root, name);
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read workspace config at {}", path.display()))?;
        toml::from_str(&content).context("Failed to parse .workspace.toml")
    }

    pub fn save(&self, root: &Path) -> Result<()> {
        let dir = Self::workspace_dir(root, &self.workspace.name);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(".workspace.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// List all workspace names by scanning .workspaces/ subdirectories
    pub fn list_all(root: &Path) -> Result<Vec<String>> {
        let ws_dir = root.join(".workspaces");
        if !ws_dir.exists() {
            return Ok(vec![]);
        }

        let mut names = Vec::new();
        for entry in std::fs::read_dir(&ws_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                // Skip hidden dirs and check for .workspace.toml
                if !name.starts_with('.') && entry.path().join(".workspace.toml").exists() {
                    names.push(name);
                }
            }
        }
        names.sort();
        Ok(names)
    }
}
