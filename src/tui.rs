use std::io;
use std::path::PathBuf;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::*,
};

use crate::config::Config;
use crate::workspace::WorkspaceConfig;
use crate::git;
use crate::commands;

// ── Data types ──────────────────────────────────────────────

#[derive(Clone)]
struct RepoStatus {
    name: String,
    branch: String,
    ahead: usize,
    dirty: usize,
    exists: bool,
}

struct WorkspaceInfo {
    name: String,
    created: String,
    repos: Vec<RepoStatus>,
}

// ── Input mode ──────────────────────────────────────────────

enum InputMode {
    Normal,
    NewName,           // typing workspace name
    NewRepos,          // selecting repos for new workspace
    Confirm(Action),   // confirm an action
}

enum Action {
    Done(String),
}

// ── App state ───────────────────────────────────────────────

struct App {
    root: PathBuf,
    config: Config,
    workspaces: Vec<WorkspaceInfo>,
    ws_index: usize,
    input_mode: InputMode,
    input_buf: String,
    repo_toggles: Vec<(String, bool)>,  // for new workspace repo selection
    repo_cursor: usize,
    message: Option<(String, bool)>,    // (msg, is_error)
    should_quit: bool,
}

impl App {
    fn new(root: PathBuf, config: Config) -> Result<Self> {
        let mut app = App {
            root,
            config,
            workspaces: Vec::new(),
            ws_index: 0,
            input_mode: InputMode::Normal,
            input_buf: String::new(),
            repo_toggles: Vec::new(),
            repo_cursor: 0,
            message: None,
            should_quit: false,
        };
        app.refresh()?;
        Ok(app)
    }

    fn refresh(&mut self) -> Result<()> {
        let names = WorkspaceConfig::list_all(&self.root)?;
        self.workspaces.clear();

        for name in &names {
            if let Ok(ws) = WorkspaceConfig::load(&self.root, name) {
                let ws_dir = WorkspaceConfig::workspace_dir(&self.root, name);
                let mut repos = Vec::new();

                for (repo_name, repo_info) in &ws.repos {
                    let wt_path = ws_dir.join(repo_name);
                    let exists = wt_path.exists();
                    let default_branch = self.config.repos.get(repo_name)
                        .map(|r| r.default_branch.as_str())
                        .unwrap_or("main");

                    let (ahead, dirty) = if exists {
                        (
                            git::commits_ahead(&wt_path, default_branch).unwrap_or(0),
                            git::dirty_count(&wt_path).unwrap_or(0),
                        )
                    } else {
                        (0, 0)
                    };

                    repos.push(RepoStatus {
                        name: repo_name.clone(),
                        branch: repo_info.branch.clone(),
                        ahead,
                        dirty,
                        exists,
                    });
                }

                self.workspaces.push(WorkspaceInfo {
                    name: name.clone(),
                    created: ws.workspace.created,
                    repos,
                });
            }
        }

        // Clamp index
        if !self.workspaces.is_empty() && self.ws_index >= self.workspaces.len() {
            self.ws_index = self.workspaces.len() - 1;
        }

        Ok(())
    }

    fn selected_ws(&self) -> Option<&WorkspaceInfo> {
        self.workspaces.get(self.ws_index)
    }

    fn set_msg(&mut self, msg: impl Into<String>, is_error: bool) {
        self.message = Some((msg.into(), is_error));
    }

    fn start_new_workspace(&mut self) {
        self.input_buf.clear();
        self.input_mode = InputMode::NewName;
        self.message = Some(("Enter workspace name".into(), false));
    }

    fn start_repo_selection(&mut self) {
        self.repo_toggles = self.config.repos.keys()
            .map(|name| (name.clone(), false))
            .collect();
        self.repo_cursor = 0;
        self.input_mode = InputMode::NewRepos;
        self.message = Some(("Space=toggle, Enter=create, Esc=cancel".into(), false));
    }

    fn create_workspace(&mut self) -> Result<()> {
        let name = self.input_buf.clone();
        let repos: Vec<String> = self.repo_toggles.iter()
            .filter(|(_, on)| *on)
            .map(|(name, _)| name.clone())
            .collect();

        if repos.is_empty() {
            self.set_msg("Select at least one repo", true);
            return Ok(());
        }

        commands::new::run(&name, &repos, None)?;
        self.refresh()?;
        // Select the newly created workspace
        if let Some(idx) = self.workspaces.iter().position(|w| w.name == name) {
            self.ws_index = idx;
        }
        self.set_msg(format!("Created workspace '{}'", name), false);
        Ok(())
    }

    fn open_editor(&self, editor: &str) -> Result<()> {
        if let Some(ws) = self.selected_ws() {
            commands::open::run(&ws.name, editor)?;
        }
        Ok(())
    }

    fn open_claude_new_tab(&mut self) -> Result<()> {
        if let Some(ws) = self.selected_ws() {
            commands::open::run_claude_new_tab(&ws.name)?;
            self.set_msg(format!("Opening Claude in new tab for '{}'", ws.name), false);
        }
        Ok(())
    }

    fn open_finder(&mut self) -> Result<()> {
        if let Some(ws) = self.selected_ws() {
            commands::open::run_finder(&ws.name)?;
            self.set_msg(format!("Opened '{}' in Finder", ws.name), false);
        }
        Ok(())
    }

    fn done_workspace(&mut self) -> Result<()> {
        if let Some(ws) = self.selected_ws() {
            let name = ws.name.clone();
            self.input_mode = InputMode::Confirm(Action::Done(name));
            self.message = Some(("Delete this workspace? y/n".into(), true));
        }
        Ok(())
    }

    fn confirm_action(&mut self) -> Result<()> {
        if let InputMode::Confirm(Action::Done(ref name)) = self.input_mode {
            let name = name.clone();
            commands::done::run(&name, true)?;
            self.input_mode = InputMode::Normal;
            self.refresh()?;
            self.set_msg(format!("Workspace '{}' removed", name), false);
        }
        Ok(())
    }
}

// ── Terminal lifecycle ──────────────────────────────────────

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

// ── Main loop ───────────────────────────────────────────────

pub fn run() -> Result<()> {
    let root = Config::find_root()?;
    let config = Config::load(&root)?;
    let mut app = App::new(root, config)?;

    let mut terminal = setup_terminal()?;

    loop {
        terminal.draw(|f| draw(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match &app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.should_quit = true,
                    KeyCode::Char('j') | KeyCode::Down => {
                        if !app.workspaces.is_empty() {
                            app.ws_index = (app.ws_index + 1).min(app.workspaces.len() - 1);
                            app.message = None;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if app.ws_index > 0 {
                            app.ws_index -= 1;
                            app.message = None;
                        }
                    }
                    KeyCode::Char('n') => app.start_new_workspace(),
                    KeyCode::Char('o') => {
                        if app.selected_ws().is_some() {
                            restore_terminal(&mut terminal)?;
                            let _ = app.open_editor("zed");
                            terminal = setup_terminal()?;
                        }
                    }
                    KeyCode::Char('c') => {
                        if app.selected_ws().is_some() {
                            let _ = app.open_claude_new_tab();
                        }
                    }
                    KeyCode::Char('f') => {
                        if app.selected_ws().is_some() {
                            let _ = app.open_finder();
                        }
                    }
                    KeyCode::Char('d') => {
                        let _ = app.done_workspace();
                    }
                    KeyCode::Char('r') => {
                        app.refresh()?;
                        app.set_msg("Refreshed", false);
                    }
                    _ => {}
                },
                InputMode::NewName => match key.code {
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                        app.message = None;
                    }
                    KeyCode::Enter => {
                        if app.input_buf.is_empty() {
                            app.set_msg("Name cannot be empty", true);
                        } else {
                            app.start_repo_selection();
                        }
                    }
                    KeyCode::Backspace => { app.input_buf.pop(); }
                    KeyCode::Char(c) => {
                        // Only allow valid directory name chars
                        if c.is_alphanumeric() || c == '-' || c == '_' {
                            app.input_buf.push(c);
                        }
                    }
                    _ => {}
                },
                InputMode::NewRepos => match key.code {
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                        app.message = None;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        if !app.repo_toggles.is_empty() {
                            app.repo_cursor = (app.repo_cursor + 1).min(app.repo_toggles.len() - 1);
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if app.repo_cursor > 0 {
                            app.repo_cursor -= 1;
                        }
                    }
                    KeyCode::Char(' ') => {
                        if let Some(toggle) = app.repo_toggles.get_mut(app.repo_cursor) {
                            toggle.1 = !toggle.1;
                        }
                    }
                    KeyCode::Enter => {
                        let result = app.create_workspace();
                        match result {
                            Ok(()) => { app.input_mode = InputMode::Normal; }
                            Err(e) => { app.set_msg(format!("Error: {}", e), true); }
                        }
                    }
                    _ => {}
                },
                InputMode::Confirm(_) => match key.code {
                    KeyCode::Char('y') => {
                        if let Err(e) = app.confirm_action() {
                            app.set_msg(format!("Error: {}", e), true);
                            app.input_mode = InputMode::Normal;
                        }
                    }
                    _ => {
                        app.input_mode = InputMode::Normal;
                        app.message = None;
                    }
                },
            }
        }

        if app.should_quit {
            restore_terminal(&mut terminal)?;
            break;
        }
    }

    Ok(())
}

// ── Drawing ─────────────────────────────────────────────────

fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(3), // status bar
        ])
        .split(f.area());

    let main_area = chunks[0];
    let status_area = chunks[1];

    // Main split: left panel (workspace list) | right panel (details)
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        ])
        .split(main_area);

    draw_workspace_list(f, app, panels[0]);
    draw_detail_panel(f, app, panels[1]);
    draw_status_bar(f, app, status_area);

    // Draw overlay for input modes
    match &app.input_mode {
        InputMode::NewName => draw_name_input(f, app),
        InputMode::NewRepos => draw_repo_selector(f, app),
        InputMode::Confirm(_) => {} // shown in status bar
        InputMode::Normal => {}
    }
}

fn draw_workspace_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app.workspaces.iter().enumerate().map(|(i, ws)| {
        let repo_count = ws.repos.len();
        let dirty: usize = ws.repos.iter().map(|r| r.dirty).sum();
        let ahead: usize = ws.repos.iter().map(|r| r.ahead).sum();

        let mut status_parts = vec![format!("{} repos", repo_count)];
        if ahead > 0 { status_parts.push(format!("{}↑", ahead)); }
        if dirty > 0 { status_parts.push(format!("{}✎", dirty)); }

        let style = if i == app.ws_index {
            Style::default().fg(Color::Black).bg(Color::Cyan).bold()
        } else {
            Style::default()
        };

        ListItem::new(Line::from(vec![
            Span::styled(
                format!(" {} ", ws.name),
                style,
            ),
            Span::styled(
                format!(" {}", status_parts.join(" ")),
                if i == app.ws_index {
                    Style::default().fg(Color::DarkGray).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
        ]))
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Workspaces ")
            .title_style(Style::default().bold()));

    f.render_widget(list, area);
}

fn draw_detail_panel(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ws) = app.selected_ws() {
        let inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),   // header
                Constraint::Min(5),      // repo table
                Constraint::Length(3),   // keybindings
            ])
            .split(area);

        // Header
        let header = Paragraph::new(Line::from(vec![
            Span::styled(&ws.name, Style::default().bold().fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled(format!("created {}", ws.created), Style::default().fg(Color::DarkGray)),
        ]))
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded));
        f.render_widget(header, inner[0]);

        // Repo table
        let header_row = Row::new(vec!["Repo", "Branch", "Status"])
            .style(Style::default().bold().fg(Color::Yellow))
            .bottom_margin(1);

        let rows: Vec<Row> = ws.repos.iter().map(|repo| {
            let status = if !repo.exists {
                "MISSING".to_string()
            } else {
                let mut parts = Vec::new();
                if repo.ahead > 0 { parts.push(format!("{} ahead", repo.ahead)); }
                if repo.dirty > 0 { parts.push(format!("{} dirty", repo.dirty)); }
                if parts.is_empty() { parts.push("clean".to_string()); }
                parts.join(", ")
            };

            let status_style = if !repo.exists {
                Style::default().fg(Color::Red)
            } else if repo.dirty > 0 {
                Style::default().fg(Color::Yellow)
            } else if repo.ahead > 0 {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::Green)
            };

            // Truncate branch for display
            let branch_display = if repo.branch.len() > 30 {
                format!("{}…", &repo.branch[..29])
            } else {
                repo.branch.clone()
            };

            Row::new(vec![
                Cell::from(repo.name.clone()).style(Style::default().bold()),
                Cell::from(branch_display).style(Style::default().fg(Color::DarkGray)),
                Cell::from(status).style(status_style),
            ])
        }).collect();

        let table = Table::new(
            rows,
            [Constraint::Percentage(30), Constraint::Percentage(35), Constraint::Percentage(35)],
        )
        .header(header_row)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Repos "));
        f.render_widget(table, inner[1]);

        // Keybindings
        let keys = Line::from(vec![
            Span::styled(" n", Style::default().bold().fg(Color::Cyan)),
            Span::raw("ew  "),
            Span::styled("o", Style::default().bold().fg(Color::Cyan)),
            Span::raw("pen  "),
            Span::styled("c", Style::default().bold().fg(Color::Cyan)),
            Span::raw("laude  "),
            Span::styled("f", Style::default().bold().fg(Color::Cyan)),
            Span::raw("inder  "),
            Span::styled("d", Style::default().bold().fg(Color::Cyan)),
            Span::raw("one  "),
            Span::styled("r", Style::default().bold().fg(Color::Cyan)),
            Span::raw("efresh  "),
            Span::styled("q", Style::default().bold().fg(Color::Cyan)),
            Span::raw("uit"),
        ]);
        let keybindings = Paragraph::new(keys)
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded));
        f.render_widget(keybindings, inner[2]);
    } else {
        // Empty state
        let empty = Paragraph::new(vec![
            Line::raw(""),
            Line::raw("  No workspaces yet."),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  Press "),
                Span::styled("n", Style::default().bold().fg(Color::Cyan)),
                Span::raw(" to create a new workspace."),
            ]),
        ])
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Details "));
        f.render_widget(empty, area);
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let (content, style) = if let Some((ref msg, ref is_error)) = app.message {
        let style = if *is_error {
            Style::default().fg(Color::Red).bold()
        } else {
            Style::default().fg(Color::Green)
        };
        (msg.clone(), style)
    } else {
        (format!("iws — {} workspace(s)", app.workspaces.len()), Style::default().fg(Color::DarkGray))
    };

    let bar = Paragraph::new(content)
        .style(style)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded));
    f.render_widget(bar, area);
}

fn draw_name_input(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 5, f.area());

    f.render_widget(Clear, area);

    let input = Paragraph::new(Line::from(vec![
        Span::raw(&app.input_buf),
        Span::styled("│", Style::default().fg(Color::Cyan)),
    ]))
    .block(Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" New Workspace Name ")
        .title_style(Style::default().bold().fg(Color::Cyan)));
    f.render_widget(input, area);
}

fn draw_repo_selector(f: &mut Frame, app: &App) {
    let height = (app.repo_toggles.len() + 4).min(15) as u16;
    let area = centered_rect(50, height, f.area());

    f.render_widget(Clear, area);

    let items: Vec<ListItem> = app.repo_toggles.iter().enumerate().map(|(i, (name, on))| {
        let marker = if *on { "[x]" } else { "[ ]" };
        let style = if i == app.repo_cursor {
            Style::default().fg(Color::Black).bg(Color::Cyan).bold()
        } else {
            Style::default()
        };
        ListItem::new(format!(" {} {}", marker, name)).style(style)
    }).collect();

    let title = format!(" Select repos for '{}' ", app.input_buf);
    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan))
            .title(title)
            .title_style(Style::default().bold().fg(Color::Cyan)));
    f.render_widget(list, area);
}

fn centered_rect(width_pct: u16, height: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_pct) / 2),
            Constraint::Percentage(width_pct),
            Constraint::Percentage((100 - width_pct) / 2),
        ])
        .split(popup_layout[1])[1]
}
