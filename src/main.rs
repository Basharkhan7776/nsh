use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

struct App {
    output_lines: Vec<String>,
    input: String,
    cursor_position: usize,
    scroll_offset: usize,
}

impl App {
    fn new() -> Self {
        Self {
            output_lines: Vec::new(),
            input: String::new(),
            cursor_position: 0,
            scroll_offset: 0,
        }
    }

    fn add_line(&mut self, line: &str) {
        self.output_lines.push(line.to_string());
        self.scroll_to_bottom();
    }

    fn scroll_to_bottom(&mut self) {
        let max_scroll = self.output_lines.len().saturating_sub(1);
        self.scroll_offset = max_scroll;
    }

    fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    fn scroll_down(&mut self) {
        let max_scroll = self.output_lines.len().saturating_sub(1);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }
}

fn execute_command(input: &str, app: &mut App, _cwd: &str) {
    let input = input.trim();
    if input.is_empty() {
        return;
    }

    let mut parts = input.split_whitespace();
    let program = parts.next().unwrap();
    let args: Vec<&str> = parts.collect();

    match program {
        "exit" | "quit" => {}
        "cd" => {
            let target = if args.is_empty() {
                std::env::var("HOME").unwrap_or_else(|_| String::from("/"))
            } else {
                args[0].to_string()
            };
            if let Err(e) = std::env::set_current_dir(&target) {
                app.add_line(&format!("\x1b[1;31mcd: {}: {}\x1b[0m", target, e));
            }
        }
        "clear" => {
            app.output_lines.clear();
            app.scroll_offset = 0;
            return;
        }
        "help" => {
            app.add_line("\x1b[1;36mAvailable commands:\x1b[0m");
            app.add_line("  \x1b[31mask\x1b[0m <question>  - Ask AI");
            app.add_line("  \x1b[31mdo\x1b[0m <task>      - Execute task");
            app.add_line("  \x1b[31mplan\x1b[0m <goal>      - Plan goal");
            app.add_line("  \x1b[31mbuild\x1b[0m <project>  - Build project");
            app.add_line("  \x1b[31mcd\x1b[0m <dir>        - Change directory");
            app.add_line("  \x1b[31mclear\x1b[0m          - Clear screen");
            app.add_line("  \x1b[31mexit\x1b[0m / \x1b[31mquit\x1b[0m   - Exit shell");
            return;
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
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                app.add_line(&line);
                            }
                        }
                    }
                    if let Some(stderr) = child.stderr.take() {
                        let reader = BufReader::new(stderr);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                app.add_line(&format!("\x1b[1;31m{}\x1b[0m", line));
                            }
                        }
                    }
                    child.wait().ok();
                }
                Err(_e) => {
                    app.add_line(&format!("\x1b[1;31m{}: command not found\x1b[0m", program));
                }
            }
            return;
        }
    }
}

fn main() -> std::io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    app.add_line("\x1b[1;36mWelcome to nsh - AI-Powered Shell\x1b[0m");
    app.add_line("\x1b[90mType 'help' for commands\x1b[0m");
    app.add_line("");

    let mut running = true;

    while running {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "~".to_string());

        terminal.draw(|f| {
            let black = Style::default().bg(Color::Black);
            let red = Style::default().fg(Color::Red).bg(Color::Black);
            let white = Style::default().fg(Color::White).bg(Color::Black);
            let _gray = Style::default().fg(Color::DarkGray).bg(Color::Black);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(3)])
                .split(f.area());

            let output_area = chunks[0];
            let input_area = chunks[1];

            let visible_lines: Vec<Line> = app
                .output_lines
                .iter()
                .skip(app.scroll_offset)
                .map(|line| Line::from(line.as_str()))
                .collect();

            let output_widget = Paragraph::new(visible_lines)
                .block(
                    Block::default()
                        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                        .border_style(red),
                )
                .style(black)
                .scroll((app.scroll_offset as u16, 0));
            f.render_widget(output_widget, output_area);

            let prompt_len = cwd.len() + 4;
            let input_with_prompt =
                format!("\x1b[1;31m{}\x1b[0m\x1b[90m $\x1b[0m {}", cwd, app.input);
            let input_widget = Paragraph::new(input_with_prompt.as_str())
                .block(Block::default().borders(Borders::ALL).border_style(red))
                .style(white);
            f.render_widget(input_widget, input_area);

            let cursor_x = (prompt_len + app.cursor_position) as u16;
            f.set_cursor_position(ratatui::layout::Position {
                x: cursor_x.min(input_area.width.saturating_sub(1)),
                y: input_area.y + 1,
            });
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char(c) => {
                    if key.modifiers.contains(event::KeyModifiers::CONTROL) {
                        if c == 'c' {
                            app.add_line("");
                            app.add_line("\x1b[90m^C\x1b[0m");
                            app.input.clear();
                            app.cursor_position = 0;
                        } else if c == 'd' {
                            if app.input.is_empty() {
                                running = false;
                            }
                        }
                    } else {
                        app.input.insert(app.cursor_position, c);
                        app.cursor_position += 1;
                    }
                }
                KeyCode::Backspace => {
                    if app.cursor_position > 0 {
                        app.cursor_position -= 1;
                        app.input.remove(app.cursor_position);
                    }
                }
                KeyCode::Delete => {
                    if app.cursor_position < app.input.len() {
                        app.input.remove(app.cursor_position);
                    }
                }
                KeyCode::Left => {
                    if app.cursor_position > 0 {
                        app.cursor_position -= 1;
                    }
                }
                KeyCode::Right => {
                    if app.cursor_position < app.input.len() {
                        app.cursor_position += 1;
                    }
                }
                KeyCode::Enter => {
                    let cwd = std::env::current_dir()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| "~".to_string());
                    app.add_line(&format!(
                        "\x1b[1;31m{}\x1b[0m\x1b[90m $\x1b[0m {}",
                        cwd, app.input
                    ));

                    let input = app.input.clone();

                    if input == "exit" || input == "quit" {
                        running = false;
                    } else if !input.is_empty() {
                        execute_command(&input, &mut app, &cwd);
                    }

                    app.input.clear();
                    app.cursor_position = 0;
                }
                KeyCode::Up => {
                    app.scroll_up();
                }
                KeyCode::Down => {
                    app.scroll_down();
                }
                KeyCode::PageUp => {
                    for _ in 0..10 {
                        app.scroll_up();
                    }
                }
                KeyCode::PageDown => {
                    for _ in 0..10 {
                        app.scroll_down();
                    }
                }
                KeyCode::Home => {
                    app.cursor_position = 0;
                }
                KeyCode::End => {
                    app.cursor_position = app.input.len();
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
