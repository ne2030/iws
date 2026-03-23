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

/// Open Claude Code in a new Warp tab (non-blocking, TUI stays alive)
pub fn run_claude_new_tab(name: &str) -> Result<()> {
    let root = Config::find_root()?;
    let _ws = WorkspaceConfig::load(&root, name)?;
    let ws_dir = WorkspaceConfig::workspace_dir(&root, name);
    let ws_path = ws_dir.to_string_lossy();

    let script = format!(
        r#"
tell application "Warp" to activate
delay 0.3
tell application "System Events"
    keystroke "t" using command down
    delay 0.5
    keystroke "cd {} && claude"
    key code 36
end tell
"#,
        shell_escape(&ws_path),
    );

    std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .spawn()?;

    Ok(())
}

/// Open workspace directory in Finder
pub fn run_finder(name: &str) -> Result<()> {
    let root = Config::find_root()?;
    let _ws = WorkspaceConfig::load(&root, name)?;
    let ws_dir = WorkspaceConfig::workspace_dir(&root, name);

    std::process::Command::new("open")
        .arg(&ws_dir)
        .spawn()?;

    Ok(())
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
