# iws

**Multi-repo workspace manager** — git worktree orchestration for coordinated changes across multiple repositories.

## Why iws?

When working on features that span multiple repositories (e.g., backend + frontend + shared libs), you need isolated branches and working directories for each. Manually managing git worktrees across repos is tedious and error-prone.

iws automates the entire lifecycle:

```
iws init → iws new → work on code → iws pr → iws done
```

Each workspace creates isolated git worktrees for selected repos, so your main branches stay clean.

## Features

- **Workspace lifecycle** — create, manage, and clean up multi-repo worktrees with a single command
- **Interactive TUI** — browse workspaces, view status, and manage repos with keyboard navigation
- **Smart PR creation** — generate pull requests across all repos with automatic cross-references
- **Editor integration** — open workspaces in Zed, VS Code, or Cursor
- **Claude Code integration** — launch Claude Code sessions scoped to workspace context
- **Shared files** — copy common config files (`.env`, keyfiles) to each worktree
- **SPEC.md templates** — auto-generated workspace specification for tracking requirements

## Installation

### From source

```bash
git clone https://github.com/ne2030/iws
cd iws
cargo build --release
```

The binary is at `./target/release/iws`. Move it to your `$PATH`:

```bash
cp target/release/iws ~/.local/bin/
```

### Requirements

- Rust 1.70+ (for building)
- Git 2.20+ (worktree support)
- [gh CLI](https://cli.github.com/) (optional, for `iws pr`)

## Quick Start

```bash
# 1. Navigate to your multi-repo project root
cd ~/projects/my-platform
#    my-platform/
#    ├── backend/      (git repo)
#    ├── frontend/     (git repo)
#    └── shared-lib/   (git repo)

# 2. Initialize iws (auto-detects git repos)
iws init

# 3. Create a workspace
iws new auth-refactor --repos backend,frontend

# 4. Work on your code in .workspaces/auth-refactor/{repo}/
#    Edit .workspaces/auth-refactor/SPEC.md to track requirements

# 5. Create PRs across all repos
iws pr auth-refactor

# 6. Clean up when done
iws done auth-refactor
```

## Usage

### Interactive TUI

```bash
iws
```

Launch the terminal UI for browsing and managing workspaces with keyboard navigation.

### CLI Commands

| Command | Description |
|---------|-------------|
| `iws init` | Initialize iws in current directory |
| `iws new <name> --repos r1,r2 [--branch name]` | Create workspace with worktrees |
| `iws list` | List all workspaces |
| `iws status <name>` | Show workspace status (commits ahead, dirty files) |
| `iws open <name> [--editor zed\|code\|cursor]` | Open in editor |
| `iws claude <name>` | Start Claude Code session in workspace |
| `iws add-repo <name> <repo> [--branch name]` | Add repo to workspace |
| `iws remove-repo <name> <repo>` | Remove repo from workspace |
| `iws pr <name>` | Create PRs for all repos |
| `iws done <name> [--force]` | Clean up workspace |

## How It Works

### Project Structure

After `iws init`, a `.workspaces/` directory is created:

```
my-platform/
├── backend/
├── frontend/
└── .workspaces/
    ├── config.toml              # Project config (repos, shared files)
    └── auth-refactor/           # Workspace
        ├── .workspace.toml      # Workspace config
        ├── SPEC.md              # Requirements & progress
        ├── .claude/CLAUDE.md    # Claude Code context
        ├── backend/             # Git worktree → backend repo
        └── frontend/            # Git worktree → frontend repo
```

### Workspace Lifecycle

1. **`init`** — Scans current directory for git repos, creates `.workspaces/config.toml`
2. **`new`** — Creates worktrees with `feature/{name}` branches (or custom), copies shared files, generates SPEC.md
3. **Work** — Edit code in worktree directories. Main repo checkouts are untouched
4. **`pr`** — Pushes branches and creates GitHub PRs with cross-references between repos
5. **`done`** — Removes worktrees and local branches (warns about uncommitted changes)

## Configuration

### Project Config (`.workspaces/config.toml`)

```toml
[project]
name = "my-platform"
root = "/path/to/my-platform"

[repos.backend]
path = "backend"
default_branch = "main"
shared_files = [".env"]

[repos.frontend]
path = "frontend"
default_branch = "main"
shared_files = [".env.local"]
```

### Workspace Config (`.workspaces/{name}/.workspace.toml`)

```toml
[workspace]
name = "auth-refactor"
created = "2024-03-20"

[repos.backend]
branch = "feature/auth-refactor"

[repos.frontend]
branch = "feature/auth-refactor"
```

## License

MIT
