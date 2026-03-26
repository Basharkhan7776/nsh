use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
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
    total_lines: usize,
    current_suggestions: Vec<String>,
    show_suggestions: bool,
    selected_suggestion: usize,
    saved_input: String,
    history_index: Option<usize>,
}

impl App {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            current_input: String::new(),
            cursor_position: 0,
            scroll_offset: 0,
            total_lines: 0,
            current_suggestions: Vec::new(),
            show_suggestions: false,
            selected_suggestion: 0,
            saved_input: String::new(),
            history_index: None,
        }
    }

    fn add_entry(&mut self, entry: Entry) {
        self.entries.push(entry);
        self.recalc_total_lines();
        self.scroll_to_bottom();
    }

    fn recalc_total_lines(&mut self) {
        self.total_lines = self.entries.iter().map(|e| e.content.len()).sum();
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.total_lines = 0;
        self.scroll_offset = 0;
    }

    fn scroll_to_bottom(&mut self) {
        let visible = self.visible_count();
        self.scroll_offset = self.total_lines.saturating_sub(visible);
    }

    fn visible_count(&self) -> usize {
        20
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

        if self.current_input.starts_with("cd ") {
            let dir_part = self.current_input[3..].trim();
            if let Ok(entries) = std::fs::read_dir(".") {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        let name_lower = name.to_lowercase();
                        if name_lower.starts_with(&dir_part.to_lowercase()) || dir_part.is_empty() {
                            let prefix = if is_dir { "" } else { "" };
                            suggestions.push(format!("{}{}", prefix, name));
                        }
                    }
                }
            }
        } else {
            for cmd in PATH_COMMANDS.iter() {
                if cmd.to_lowercase().starts_with(&input_lower) {
                    suggestions.push(cmd.clone());
                }
            }

            for entry in self.entries.iter().rev() {
                if entry.entry_type == EntryType::Command {
                    if let Some(cmd) = entry.content.first() {
                        if cmd.to_lowercase().starts_with(&input_lower)
                            && !suggestions.contains(cmd)
                        {
                            suggestions.push(cmd.clone());
                        }
                    }
                }
                if suggestions.len() >= 10 {
                    break;
                }
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
        let output_bg = Style::default().bg(Color::Black);
        let output_fg = Style::default().fg(Color::White).bg(Color::Black);
        let prompt_fg = Style::default().fg(Color::Green).bg(Color::Black);
        let input_prompt_fg = Style::default().fg(Color::Green).bg(Color::Rgb(30, 30, 30));
        let gray = Style::default()
            .fg(Color::DarkGray)
            .bg(Color::Rgb(30, 30, 30));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(f.area());

        let list_area = chunks[0];
        let input_area = chunks[1];

        let visible_height = list_area.height as usize;
        let content_height = app.entries.iter().map(|e| e.content.len()).sum::<usize>();
        let start_line = app.scroll_offset;
        let end_line = (start_line + visible_height).min(content_height);

        let mut current_line = 0;
        let mut items: Vec<ListItem> = Vec::new();

        for entry in &app.entries {
            let entry_height = entry.content.len();
            let entry_end = current_line + entry_height;

            if entry_end <= start_line {
                current_line = entry_end;
                continue;
            }

            let skip = if current_line < start_line {
                start_line - current_line
            } else {
                0
            };
            let show_from = current_line + skip;
            let show_to = entry_end.min(end_line);

            for i in show_from..show_to {
                let line_idx = i - current_line;
                if let Some(line) = entry.content.get(line_idx) {
                    match entry.entry_type {
                        EntryType::Command => {
                            if let Some(cmd) = entry.content.first() {
                                if i == current_line {
                                    let display = format!("{} $ {}", entry.cwd, cmd);
                                    items.push(ListItem::new(Line::from(Span::styled(
                                        display, prompt_fg,
                                    ))));
                                }
                            }
                        }
                        EntryType::Output => {
                            items.push(ListItem::new(Line::from(Span::styled(line, output_fg))));
                        }
                        EntryType::System => {
                            items.push(ListItem::new(Line::from(Span::styled(line, gray))));
                        }
                    }
                }
            }

            current_line = entry_end;
            if current_line >= end_line {
                break;
            }
        }

        let list = List::new(items).style(output_bg);
        f.render_widget(list, list_area);

        let input_with_prompt = format!("$ {}", app.current_input);
        let input_widget = Paragraph::new(input_with_prompt.as_str()).style(input_prompt_fg);
        f.render_widget(input_widget, input_area);

        let cursor_x = (1 + app.cursor_position) as u16;
        let cursor_y = input_area.y + 1;
        f.set_cursor_position(ratatui::layout::Position {
            x: cursor_x.min(input_area.width.saturating_sub(1)),
            y: cursor_y,
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
            let suggestions_list = List::new(suggestions_items).style(gray);
            let suggestions_area = Rect {
                x: 2,
                y: input_area.y.saturating_sub(suggestions_height),
                width: 30.min(input_area.width - 2),
                height: suggestions_height,
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
    app.add_entry(Entry {
        entry_type: EntryType::System,
        content: vec![
            "Welcome to nsh - AI-Powered Shell".to_string(),
            "Type 'help' for commands".to_string(),
            "Use Tab for autocomplete, Up/Down for history".to_string(),
        ],
        cwd: "~".to_string(),
    });

    let mut running = true;

    while running {
        render(&mut terminal, &app)?;

        loop {
            match event::poll(std::time::Duration::from_millis(50)) {
                Ok(true) => {
                    if let Ok(event) = event::read() {
                        match event {
                            Event::Key(key) => {
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
                                                app.add_entry(Entry {
                                                    entry_type: EntryType::Command,
                                                    content: vec!["^C".to_string()],
                                                    cwd: cwd.clone(),
                                                });
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
                                            app.add_entry(Entry {
                                                entry_type: EntryType::Command,
                                                content: vec![input.clone()],
                                                cwd: cwd.clone(),
                                            });
                                            let output = execute_command(&input);
                                            if output.iter().any(|s| s == "__CLEAR__") {
                                                app.clear();
                                            } else if !output.is_empty() {
                                                app.add_entry(Entry {
                                                    entry_type: EntryType::Output,
                                                    content: output,
                                                    cwd: String::new(),
                                                });
                                            }
                                        }
                                        app.current_input.clear();
                                        app.cursor_position = 0;
                                        app.saved_input.clear();
                                        app.history_index = None;
                                        app.show_suggestions = false;
                                        app.current_suggestions.clear();
                                    }
                                    KeyCode::Tab => {
                                        if !app.current_suggestions.is_empty() {
                                            if let Some(s) =
                                                app.current_suggestions.get(app.selected_suggestion)
                                            {
                                                if app.current_input.starts_with("cd ") {
                                                    app.current_input = format!("cd {}", s);
                                                } else {
                                                    app.current_input = s.clone();
                                                }
                                                app.cursor_position = app.current_input.len();
                                            }
                                            app.show_suggestions = false;
                                            app.current_suggestions.clear();
                                        } else if !app.current_input.is_empty() {
                                            app.update_suggestions();
                                            if app.current_suggestions.len() == 1 {
                                                if app.current_input.starts_with("cd ") {
                                                    app.current_input = format!(
                                                        "cd {}",
                                                        app.current_suggestions[0]
                                                    );
                                                } else {
                                                    app.current_input =
                                                        app.current_suggestions[0].clone();
                                                }
                                                app.cursor_position = app.current_input.len();
                                                app.show_suggestions = false;
                                                app.current_suggestions.clear();
                                            }
                                        }
                                    }
                                    KeyCode::Up => {
                                        if app.show_suggestions
                                            && !app.current_suggestions.is_empty()
                                        {
                                            app.selected_suggestion =
                                                app.selected_suggestion.saturating_sub(1);
                                        } else {
                                            let commands = app.get_history_commands();
                                            if commands.is_empty() {
                                                break;
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
                                        if app.show_suggestions
                                            && !app.current_suggestions.is_empty()
                                        {
                                            app.selected_suggestion = (app.selected_suggestion + 1)
                                                % app.current_suggestions.len();
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
                                        app.scroll_offset = app.scroll_offset.saturating_sub(5)
                                    }
                                    KeyCode::PageDown => {
                                        app.scroll_offset = (app.scroll_offset + 5)
                                            .min(app.total_lines.saturating_sub(1))
                                    }
                                    KeyCode::Home => app.scroll_offset = 0,
                                    KeyCode::End => app.scroll_to_bottom(),
                                    KeyCode::Esc => {
                                        app.show_suggestions = false;
                                        app.current_suggestions.clear();
                                    }
                                    _ => {}
                                }
                                break;
                            }
                            Event::Mouse(mouse) => {
                                if mouse.kind == MouseEventKind::ScrollUp {
                                    app.scroll_offset = app.scroll_offset.saturating_sub(3);
                                } else if mouse.kind == MouseEventKind::ScrollDown {
                                    app.scroll_offset = (app.scroll_offset + 3)
                                        .min(app.total_lines.saturating_sub(1));
                                }
                                break;
                            }
                            _ => {
                                break;
                            }
                        }
                    }
                }
                Ok(false) | Err(_) => {
                    break;
                }
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
