use std::path::Path;
use anyhow::Result;

/// Generate SPEC.md template for a new workspace
pub fn generate(workspace_dir: &Path, name: &str, repo_names: &[String]) -> Result<()> {
    let repos_section: String = repo_names
        .iter()
        .enumerate()
        .map(|(i, r)| format!("{}. **{}**: ", i + 1, r))
        .collect::<Vec<_>>()
        .join("\n");

    let checklist: String = repo_names
        .iter()
        .map(|r| format!("- [ ] {} ", r))
        .collect::<Vec<_>>()
        .join("\n");

    let content = format!(
        r#"# {name}

## 요구사항
-

## 레포별 작업
{repos_section}

## 작업 순서


## 진행 상태
{checklist}

## 참고/메모
-
"#
    );

    std::fs::write(workspace_dir.join("SPEC.md"), content)?;
    Ok(())
}
