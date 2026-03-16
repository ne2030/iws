use std::path::Path;
use anyhow::Result;

/// Generate .claude/CLAUDE.md pointer for a workspace
pub fn generate(workspace_dir: &Path, name: &str) -> Result<()> {
    let claude_dir = workspace_dir.join(".claude");
    std::fs::create_dir_all(&claude_dir)?;

    let content = format!(
        r#"# Workspace: {name}

작업 전 반드시 `SPEC.md`를 읽고 현재 요구사항과 진행 상태를 파악하세요.
각 레포 디렉토리는 독립 git worktree입니다. 커밋/PR은 레포별로 개별 처리하세요.
"#
    );

    std::fs::write(claude_dir.join("CLAUDE.md"), content)?;
    Ok(())
}
