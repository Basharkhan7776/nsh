use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::sync::LazyLock;

static PATH_COMMANDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .filter_map(|dir| {
            std::fs::read_dir(dir).ok().map(|d| {
                d.filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect::<Vec<_>>()
            })
        })
        .flatten()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
});

#[derive(Clone)]
struct Entry {
    entry_type: EntryType,
    content: Vec<String>,
    cwd: String,
}

#[derive(Clone, PartialEq)]
enum EntryType {
    Command,
    Output,
    System,
}

struct App {
    entries: Vec<Entry>,
    current_input: String,
    cursor_position: usize,
    scroll_offset: usize,
    current_suggestions: Vec<String>,
    show_suggestions: bool,
    selected_suggestion: usize,
    saved_input: String,
    history_index: Option<usize>,
    input_height: usize,
}

impl App {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            current_input: String::new(),
            cursor_position: 0,
            scroll_offset: 0,
            current_suggestions: Vec::new(),
            show_suggestions: false,
            selected_suggestion: 0,
            saved_input: String::new(),
            history_index: None,
            input_height: 3,
        }
    }

    fn add_command(&mut self, command: String, cwd: String) {
        self.entries.push(Entry {
            entry_type: EntryType::Command,
            content: vec![command],
            cwd,
        });
    }

    fn add_output(&mut self, output: Vec<String>) {
        if !output.is_empty() {
            self.entries.push(Entry {
                entry_type: EntryType::Output,
                content: output,
                cwd: String::new(),
            });
        }
    }

    fn add_system(&mut self, content: Vec<String>) {
        self.entries.push(Entry {
            entry_type: EntryType::System,
            content,
            cwd: String::new(),
        });
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.scroll_offset = 0;
    }

    fn scroll_to_bottom(&mut self) {
        let total = self.entries.len();
        let visible = self.visible_entries_count();
        self.scroll_offset = total.saturating_sub(visible);
    }

    fn visible_entries_count(&self) -> usize {
        let total_height = self.entries.len();
        total_height
    }

    fn get_history_commands(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.entry_type == EntryType::Command)
            .filter_map(|e| e.content.first().cloned())
            .collect()
    }

    fn update_suggestions(&mut self) {
        if self.current_input.is_empty() {
            self.current_suggestions.clear();
            self.show_suggestions = false;
            return;
        }

        let input_lower = self.current_input.to_lowercase();
        let mut suggestions: Vec<String> = Vec::new();

        for cmd in PATH_COMMANDS.iter() {
            if cmd.to_lowercase().starts_with(&input_lower) {
                suggestions.push(cmd.clone());
            }
        }

        for entry in self.entries.iter().rev() {
            if entry.entry_type == EntryType::Command {
                if let Some(cmd) = entry.content.first() {
                    if cmd.to_lowercase().starts_with(&input_lower) && !suggestions.contains(cmd) {
                        suggestions.push(cmd.clone());
                    }
                }
            }
            if suggestions.len() >= 10 {
                break;
            }
        }

        self.current_suggestions = suggestions;
        self.show_suggestions = !self.current_suggestions.is_empty();
        self.selected_suggestion = 0;
    }
}

fn execute_command(input: &str) -> Vec<String> {
    let input = input.trim();
    if input.is_empty() {
        return vec![];
    }

    let mut parts = input.split_whitespace();
    let program = match parts.next() {
        Some(p) => p,
        None => return vec![],
    };
    let args: Vec<&str> = parts.collect();

    match program {
        "exit" | "quit" => vec![],
        "cd" => {
            let target = if args.is_empty() {
                std::env::var("HOME").unwrap_or_else(|_| String::from("/"))
            } else {
                args[0].to_string()
            };
            if let Err(e) = std::env::set_current_dir(&target) {
                return vec![format!("cd: {}: {}", target, e)];
            }
            vec![]
        }
        "clear" => return vec!["__CLEAR__".to_string()],
        "help" => {
            return vec![
                "Available commands:".to_string(),
                "  ask <question>  - Ask AI (future)".to_string(),
                "  do <task>       - Execute task (future)".to_string(),
                "  plan <goal>     - Plan goal (future)".to_string(),
                "  build <project> - Build project (future)".to_string(),
                "  cd <dir>        - Change directory".to_string(),
                "  clear           - Clear screen".to_string(),
                "  exit / quit     - Exit shell".to_string(),
            ];
        }
        "ask" | "do" | "plan" | "build" => {
            return vec![format!(
                "{}: This feature will be implemented in a future update.",
                program
            )];
        }
        _ => {
            use std::io::BufRead;
            use std::io::BufReader;
            use std::process::{Command, Stdio};

            let child = Command::new(program)
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();

            match child {
                Ok(mut child) => {
                    let mut output = Vec::new();
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                output.push(line);
                            }
                        }
                    }
                    if let Some(stderr) = child.stderr.take() {
                        let reader = BufReader::new(stderr);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                output.push(line);
                            }
                        }
                    }
                    child.wait().ok();
                    output
                }
                Err(_e) => {
                    vec![format!("{}: command not found", program)]
                }
            }
        }
    }
}

fn render(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &App,
) -> std::io::Result<()> {
    terminal.draw(|f| {
        let black = Style::default().bg(Color::Black);
        let red = Style::default().fg(Color::Red).bg(Color::Black);
        let white = Style::default().fg(Color::White).bg(Color::Black);
        let _cyan = Style::default().fg(Color::Cyan).bg(Color::Black);
        let gray = Style::default().fg(Color::DarkGray).bg(Color::Black);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(app.input_height as u16),
            ])
            .split(f.area());

        let list_area = chunks[0];
        let input_area = chunks[1];

        let visible_height = list_area.height as usize;
        let start_idx = app.scroll_offset;
        let end_idx = (start_idx + visible_height).min(app.entries.len());

        let mut items: Vec<ListItem> = Vec::new();

        for i in start_idx..end_idx {
            if let Some(entry) = app.entries.get(i) {
                match entry.entry_type {
                    EntryType::Command => {
                        if let Some(cmd) = entry.content.first() {
                            let line =
                                format!("\x1b[1;31m{}\x1b[0m\x1b[90m $\x1b[0m {}", entry.cwd, cmd);
                            items.push(ListItem::new(Line::from(Span::raw(line))).style(white));
                        }
                    }
                    EntryType::Output => {
                        for line in &entry.content {
                            items.push(ListItem::new(Line::from(Span::raw(line))).style(white));
                        }
                    }
                    EntryType::System => {
                        for line in &entry.content {
                            items.push(ListItem::new(Line::from(Span::raw(line))).style(gray));
                        }
                    }
                }
            }
        }

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).border_style(red))
            .style(black);

        f.render_widget(list, list_area);

        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "~".to_string());
        let prompt_len = cwd.len() + 4;
        let input_with_prompt = format!(
            "\x1b[1;31m{}\x1b[0m\x1b[90m $\x1b[0m {}",
            cwd, app.current_input
        );
        let input_widget = Paragraph::new(input_with_prompt.as_str())
            .block(Block::default().borders(Borders::ALL).border_style(red))
            .style(white);
        f.render_widget(input_widget, input_area);

        let cursor_x = (prompt_len + app.cursor_position) as u16;
        f.set_cursor_position(ratatui::layout::Position {
            x: cursor_x.min(input_area.width.saturating_sub(1)),
            y: input_area.y + 1,
        });

        if app.show_suggestions && !app.current_suggestions.is_empty() {
            let suggestions_height = app.current_suggestions.len() as u16;
            let suggestions_items: Vec<ListItem> = app
                .current_suggestions
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    if i == app.selected_suggestion {
                        ListItem::new(Line::from(Span::styled(
                            s,
                            Style::default().bg(Color::Blue).fg(Color::White),
                        )))
                    } else {
                        ListItem::new(Line::from(Span::raw(s)))
                    }
                })
                .collect();
            let suggestions_list = List::new(suggestions_items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(gray)
                        .title("Completions"),
                )
                .style(gray);

            let suggestions_area = Rect {
                x: prompt_len as u16,
                y: input_area.y.saturating_sub(suggestions_height + 1),
                width: 30.min(input_area.width - prompt_len as u16),
                height: suggestions_height + 2,
            };
            f.render_widget(suggestions_list, suggestions_area);
        }
    })?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    app.add_system(vec![
        "Welcome to nsh - AI-Powered Shell".to_string(),
        "Type 'help' for commands".to_string(),
        "Use Tab for autocomplete, Up/Down for history".to_string(),
    ]);

    let mut running = true;

    while running {
        render(&mut terminal, &app)?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            let cwd = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "~".to_string());

            match key.code {
                KeyCode::Char(c) => {
                    if key.modifiers.contains(event::KeyModifiers::CONTROL) {
                        if c == 'c' {
                            app.add_command("^C".to_string(), cwd.clone());
                            app.current_input.clear();
                            app.cursor_position = 0;
                            app.history_index = None;
                            app.show_suggestions = false;
                        } else if c == 'd' {
                            if app.current_input.is_empty() {
                                running = false;
                            }
                        }
                    } else {
                        app.current_input.insert(app.cursor_position, c);
                        app.cursor_position += 1;
                        app.history_index = None;
                        app.update_suggestions();
                    }
                }
                KeyCode::Backspace => {
                    if app.cursor_position > 0 {
                        app.cursor_position -= 1;
                        app.current_input.remove(app.cursor_position);
                        app.history_index = None;
                        app.update_suggestions();
                    }
                }
                KeyCode::Delete => {
                    if app.cursor_position < app.current_input.len() {
                        app.current_input.remove(app.cursor_position);
                        app.update_suggestions();
                    }
                }
                KeyCode::Left => {
                    if app.cursor_position > 0 {
                        app.cursor_position -= 1;
                    }
                }
                KeyCode::Right => {
                    if app.cursor_position < app.current_input.len() {
                        app.cursor_position += 1;
                    }
                }
                KeyCode::Enter => {
                    let input = app.current_input.clone();

                    if input == "exit" || input == "quit" {
                        running = false;
                    } else if !input.is_empty() {
                        app.add_command(input.clone(), cwd.clone());
                        let output = execute_command(&input);

                        if output.iter().any(|s| s == "__CLEAR__") {
                            app.clear();
                        } else {
                            app.add_output(output);
                        }
                    }

                    app.current_input.clear();
                    app.cursor_position = 0;
                    app.saved_input.clear();
                    app.history_index = None;
                    app.show_suggestions = false;
                    app.current_suggestions.clear();
                    app.scroll_to_bottom();
                }
                KeyCode::Tab => {
                    if !app.current_suggestions.is_empty() {
                        if let Some(suggestion) =
                            app.current_suggestions.get(app.selected_suggestion)
                        {
                            app.current_input = suggestion.clone();
                            app.cursor_position = app.current_input.len();
                        }
                        app.show_suggestions = false;
                        app.current_suggestions.clear();
                    } else if !app.current_input.is_empty() {
                        app.update_suggestions();
                        if app.current_suggestions.len() == 1 {
                            app.current_input = app.current_suggestions[0].clone();
                            app.cursor_position = app.current_input.len();
                            app.show_suggestions = false;
                            app.current_suggestions.clear();
                        }
                    }
                }
                KeyCode::Up => {
                    if app.show_suggestions && !app.current_suggestions.is_empty() {
                        app.selected_suggestion = app.selected_suggestion.saturating_sub(1);
                    } else {
                        let commands = app.get_history_commands();
                        if commands.is_empty() {
                            continue;
                        }

                        if app.history_index.is_none() {
                            app.saved_input = app.current_input.clone();
                            app.history_index = Some(commands.len() - 1);
                        } else if let Some(idx) = app.history_index {
                            if idx > 0 {
                                app.history_index = Some(idx - 1);
                            }
                        }

                        if let Some(idx) = app.history_index {
                            if let Some(cmd) = commands.get(idx) {
                                app.current_input = cmd.clone();
                                app.cursor_position = app.current_input.len();
                            }
                        }
                        app.show_suggestions = false;
                    }
                }
                KeyCode::Down => {
                    if app.show_suggestions && !app.current_suggestions.is_empty() {
                        app.selected_suggestion =
                            (app.selected_suggestion + 1) % app.current_suggestions.len();
                    } else if let Some(idx) = app.history_index {
                        let commands = app.get_history_commands();
                        if idx < commands.len() - 1 {
                            app.history_index = Some(idx + 1);
                            if let Some(cmd) = commands.get(idx + 1) {
                                app.current_input = cmd.clone();
                                app.cursor_position = app.current_input.len();
                            }
                        } else {
                            app.current_input = app.saved_input.clone();
                            app.cursor_position = app.current_input.len();
                            app.history_index = None;
                        }
                        app.show_suggestions = false;
                    }
                }
                KeyCode::PageUp => {
                    app.scroll_offset = app.scroll_offset.saturating_sub(10);
                }
                KeyCode::PageDown => {
                    let max_scroll = app.entries.len().saturating_sub(1);
                    app.scroll_offset = (app.scroll_offset + 10).min(max_scroll);
                }
                KeyCode::Home => {
                    app.scroll_offset = 0;
                }
                KeyCode::End => {
                    app.scroll_to_bottom();
                }
                KeyCode::Esc => {
                    app.show_suggestions = false;
                    app.current_suggestions.clear();
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    println!("\nGoodbye!");

    Ok(())
}
