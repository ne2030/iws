use anyhow::{Result, bail};

use crate::config::Config;
use crate::workspace::WorkspaceConfig;

pub fn run(name: &str, editor: &str) -> Result<()> {
    let root = Config::find_root()?;
    let ws = WorkspaceConfig::load(&root, name)?;
    let ws_dir = WorkspaceConfig::workspace_dir(&root, name);

    let repo_paths: Vec<String> = ws.repos.keys()
        .map(|r| ws_dir.join(r).to_string_lossy().to_string())
        .collect();

    if repo_paths.is_empty() {
        bail!("Workspace '{}' has no repos", name);
    }

    let cmd = match editor {
        "zed" => "zed",
        "code" => "code",
        "cursor" => "cursor",
        other => other,
    };

    let status = std::process::Command::new(cmd)
        .args(&repo_paths)
        .status()?;

    if !status.success() {
        bail!("Editor '{}' exited with error", cmd);
    }

    Ok(())
}

pub fn run_claude(name: &str) -> Result<()> {
    let root = Config::find_root()?;
    let _ws = WorkspaceConfig::load(&root, name)?; // validate it exists
    let ws_dir = WorkspaceConfig::workspace_dir(&root, name);

    println!("Starting Claude Code in workspace '{}'...", name);
    println!("Directory: {}", ws_dir.display());

    let status = std::process::Command::new("claude")
        .current_dir(&ws_dir)
        .status()?;

    if !status.success() {
        bail!("Claude Code exited with error");
    }

    Ok(())
}
