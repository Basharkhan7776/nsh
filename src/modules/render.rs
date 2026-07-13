// UI rendering for terminal shell

use super::commands::shorten_cwd;
use super::config::*;
use super::state::{App, EntryType, SettingsPage};
use crate::ai::ProviderType;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Margin, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> std::io::Result<()> {
    terminal.draw(|f| {
        if app.show_settings {
            render_settings(f, app);
        } else {
            render_shell(f, app);
        }
    })?;
    Ok(())
}

// ── Text wrapping ────────────────────────────────────────────────────────────

/// Wrap a single logical line into rows that fit `width` terminal columns.
/// Prefers breaking on whitespace; hard-wraps long tokens.
fn wrap_line(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    if text.is_empty() {
        return vec![String::new()];
    }

    let mut rows: Vec<String> = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        let char_count = remaining.chars().count();
        if char_count <= width {
            rows.push(remaining.to_string());
            break;
        }

        // Byte index of the first char that would overflow `width`.
        let mut end = remaining
            .char_indices()
            .nth(width)
            .map(|(i, _)| i)
            .unwrap_or(remaining.len());

        let chunk = &remaining[..end];
        // Soft-wrap at last whitespace inside the chunk (not at position 0).
        if let Some(rel) = chunk
            .char_indices()
            .rev()
            .find(|(_, c)| c.is_whitespace())
            .map(|(i, _)| i)
        {
            if rel > 0 {
                end = rel;
                rows.push(remaining[..end].trim_end().to_string());
                remaining = remaining[end..].trim_start();
                continue;
            }
        }

        // Hard wrap mid-token.
        rows.push(remaining[..end].to_string());
        remaining = &remaining[end..];
    }

    if rows.is_empty() {
        rows.push(String::new());
    }
    rows
}

/// One painted row in the output pane (already width-wrapped).
struct DisplayRow {
    kind: EntryType,
    text: String,
    /// Command rows: optional (cwd, rest) split for coloring.
    cmd_cwd: Option<String>,
    cmd_rest: Option<String>,
}

/// Build the full list of display rows:
/// - Each history entry is on its own line(s)
/// - Command prompt is always its own line(s), then output lines follow
/// - Long lines wrap to the terminal width
fn build_display_rows(app: &App, width: usize) -> Vec<DisplayRow> {
    let width = width.max(1);
    let mut rows = Vec::new();

    for entry in &app.entries {
        match entry.entry_type {
            EntryType::Command => {
                let cmd = entry.content.first().map(|s| s.as_str()).unwrap_or("");
                let cwd_display = shorten_cwd(&entry.cwd);
                // Classic shell: `~/path$ command` on its own line(s)
                let full = format!("{cwd_display}$ {cmd}");
                let wrapped = wrap_line(&full, width);
                for (i, part) in wrapped.into_iter().enumerate() {
                    if i == 0 && part.starts_with(&cwd_display) {
                        let after_cwd = part[cwd_display.len()..].to_string();
                        rows.push(DisplayRow {
                            kind: EntryType::Command,
                            text: part,
                            cmd_cwd: Some(cwd_display.clone()),
                            cmd_rest: Some(after_cwd),
                        });
                    } else {
                        rows.push(DisplayRow {
                            kind: EntryType::Command,
                            text: part,
                            cmd_cwd: None,
                            cmd_rest: None,
                        });
                    }
                }
            }
            EntryType::Output => {
                // Each content element is already one logical line from the command.
                // Wrap each so nothing is clipped on the right.
                for line in &entry.content {
                    for part in wrap_line(line, width) {
                        rows.push(DisplayRow {
                            kind: EntryType::Output,
                            text: part,
                            cmd_cwd: None,
                            cmd_rest: None,
                        });
                    }
                }
            }
            EntryType::System => {
                for line in &entry.content {
                    for part in wrap_line(line, width) {
                        rows.push(DisplayRow {
                            kind: EntryType::System,
                            text: part,
                            cmd_cwd: None,
                            cmd_rest: None,
                        });
                    }
                }
            }
        }
    }

    rows
}

fn render_shell(f: &mut ratatui::Frame, app: &mut App) {
    let output_bg = Style::default().bg(OUTPUT_BG);
    let output_fg = Style::default().fg(OUTPUT_FG).bg(OUTPUT_BG);
    let cwd_style = Style::default().fg(CWD_FG).bg(OUTPUT_BG);
    let cmd_style = Style::default().fg(COMMAND_FG).bg(OUTPUT_BG);
    let input_style = Style::default().fg(INPUT_PROMPT_FG).bg(INPUT_BG);
    let suggestion_style = Style::default().fg(SUGGESTION_INDICATOR_FG).bg(INPUT_BG);
    let system_style = Style::default().fg(SYSTEM_FG).bg(OUTPUT_BG);
    let loading_style = Style::default()
        .fg(Color::Yellow)
        .bg(Color::Rgb(30, 30, 30))
        .add_modifier(Modifier::BOLD);

    let loading = app.ai_loading.is_some();
    let constraints = if loading {
        vec![
            Constraint::Min(1),
            Constraint::Length(1), // loading status bar
            Constraint::Length(3), // input
        ]
    } else {
        vec![Constraint::Min(1), Constraint::Length(3)]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    let list_area = chunks[0];
    let (status_area, input_area) = if loading {
        (Some(chunks[1]), chunks[2])
    } else {
        (None, chunks[1])
    };

    let visible_height = list_area.height as usize;
    let wrap_width = list_area.width as usize;

    // Build width-wrapped display rows: command on its line, then each output line.
    let display_rows = build_display_rows(app, wrap_width);
    let content_height = display_rows.len();

    // Keep scroll in sync with wrapped line count (stay at bottom if we were).
    let old_max = app.total_lines.saturating_sub(visible_height);
    let at_bottom = app.scroll_offset >= old_max;
    app.total_lines = content_height;
    let new_max = app.total_lines.saturating_sub(visible_height);
    if at_bottom {
        app.scroll_offset = new_max;
    } else {
        app.scroll_offset = app.scroll_offset.min(new_max);
    }

    let start_line = app.scroll_offset;
    let end_line = (start_line + visible_height).min(content_height);

    let mut items: Vec<ListItem> = Vec::new();
    for row in display_rows.iter().take(end_line).skip(start_line) {
        let line = match row.kind {
            EntryType::Command => {
                if let (Some(cwd), Some(rest)) = (&row.cmd_cwd, &row.cmd_rest) {
                    Line::from(vec![
                        Span::styled(cwd.clone(), cwd_style),
                        Span::styled(rest.clone(), cmd_style),
                    ])
                } else {
                    Line::from(Span::styled(row.text.as_str(), cmd_style))
                }
            }
            EntryType::Output => Line::from(Span::styled(row.text.as_str(), output_fg)),
            EntryType::System => Line::from(Span::styled(row.text.as_str(), system_style)),
        };
        items.push(ListItem::new(line));
    }

    let list = List::new(items).style(output_bg);
    f.render_widget(list, list_area);

    // Animated AI loading bar (ask / do / plan / build).
    if let (Some(area), Some(loading)) = (status_area, app.ai_loading.as_ref()) {
        let text = format!(" >  {}", loading.status_line());
        f.render_widget(Paragraph::new(text).style(loading_style), area);
    }

    // Real terminal cursor (no fake "|" glyph). Top padding is 1 row.
    // While AI is loading, dim the input and show a wait hint.
    let pad_top: u16 = 1;
    let pad_left: u16 = 0;
    let cursor_byte = app.cursor_position.min(app.current_input.len());
    let before = &app.current_input[..cursor_byte];
    let (prompt, body) = if app.ai_loading.is_some() {
        (" … ", "(AI is working — wait for response)")
    } else {
        (PROMPT_TEXT, app.current_input.as_str())
    };
    let input_line = Line::from(vec![
        Span::styled(prompt, input_style),
        Span::styled(body, Style::default().fg(OUTPUT_FG).bg(INPUT_BG)),
    ]);
    let input_block = Block::default()
        .style(input_style)
        .padding(ratatui::widgets::Padding::new(pad_left, 0, pad_top, 1));
    let input_widget = Paragraph::new(input_line)
        .style(input_style)
        .block(input_block);
    f.render_widget(input_widget, input_area);

    // Hide cursor while AI is loading (input is disabled).
    if app.ai_loading.is_none() {
        let prompt_cols = PROMPT_TEXT.chars().count() as u16;
        let text_cols = before.chars().count() as u16;
        let cursor_x = input_area
            .x
            .saturating_add(pad_left)
            .saturating_add(prompt_cols)
            .saturating_add(text_cols);
        let cursor_y = input_area.y.saturating_add(pad_top);
        let max_x = input_area
            .x
            .saturating_add(input_area.width.saturating_sub(1));
        if cursor_y < input_area.y.saturating_add(input_area.height) {
            f.set_cursor_position(Position {
                x: cursor_x.min(max_x),
                y: cursor_y,
            });
        }
    }

    if app.ai_loading.is_none() && app.show_suggestions && !app.current_suggestions.is_empty() {
        let visible = app.visible_suggestions();
        let has_more = app.has_more_suggestions();

        let display_height = if has_more {
            MAX_VISIBLE_SUGGESTIONS + 1
        } else {
            visible.len().min(MAX_VISIBLE_SUGGESTIONS)
        };
        let display_height = display_height as u16;

        let mut suggestions_items: Vec<ListItem> = visible
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let global_idx = app.suggestion_scroll_offset + i;
                if global_idx == app.selected_suggestion {
                    ListItem::new(Line::from(Span::styled(
                        s,
                        Style::default()
                            .bg(SUGGESTION_SELECTED_BG)
                            .fg(SUGGESTION_SELECTED_FG),
                    )))
                } else {
                    ListItem::new(Line::from(Span::raw(s)))
                }
            })
            .collect();

        if has_more {
            let more_item = ListItem::new(Line::from(Span::styled(
                "...",
                Style::default().fg(SUGGESTION_INDICATOR_FG),
            )));
            suggestions_items.push(more_item);
        }

        let suggestions_list = List::new(suggestions_items).style(suggestion_style);
        let suggestions_area = Rect {
            x: 2,
            y: input_area.y.saturating_sub(display_height),
            width: 40.min(input_area.width.saturating_sub(2)),
            height: display_height,
        };
        f.render_widget(suggestions_list, suggestions_area);
    }
}

fn render_settings(f: &mut ratatui::Frame, app: &App) {
    let page = app.current_settings_page();
    match page {
        SettingsPage::Home => render_home_page(f, app),
        SettingsPage::Provider => render_provider_page(f, app),
        SettingsPage::Model => render_model_page(f, app),
        SettingsPage::BaseUrl => render_baseurl_page(f, app),
        SettingsPage::ApiKey => render_apikey_page(f, app),
        SettingsPage::Enable => render_enable_page(f, app),
    }
}

fn highlight_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(OUTPUT_FG)
    }
}

fn cursor_prefix(is_selected: bool) -> &'static str {
    if is_selected { "▶ " } else { "  " }
}

fn render_home_page(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" AI Settings ")
        .title_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(OUTPUT_BG).fg(OUTPUT_FG));
    f.render_widget(block, area);

    let inner = area.inner(Margin {
        vertical: 2,
        horizontal: 3,
    });
    let state = &app.settings_state;
    let cursor = app.settings_cursor;

    let provider_str = state.provider.display_name();

    let api_key_str = if state.api_key_original.trim().is_empty() {
        "(empty — paste with Ctrl+Shift+V)".to_string()
    } else {
        let n = state.api_key_original.trim().chars().count();
        format!("•••••••••••••• ({n} chars)")
    };
    let enable_str = if state.enabled { "Yes" } else { "No" };

    let items = [
        format!("{:<12} {}", "Provider:", provider_str),
        format!("{:<12} {}", "Model:", state.model),
        format!("{:<12} {}", "Base URL:", state.base_url),
        format!("{:<12} {}", "API Key:", api_key_str),
        format!("{:<12} {}", "Enable:", enable_str),
        String::new(),
        format!("[ Save ]   [ Cancel ]"),
    ];

    for (i, line) in items.iter().enumerate() {
        let y = inner.y + i as u16;
        if y >= inner.y + inner.height {
            break;
        }
        let is_field = i < 5;
        let is_save = i == 6 && cursor == 5;
        let is_cancel = i == 6 && cursor == 6;
        let selected = (is_field && cursor == i) || is_save || is_cancel;

        let prefix = if is_field {
            cursor_prefix(cursor == i)
        } else if i == 6 {
            if cursor == 5 {
                "  ▶ "
            } else if cursor == 6 {
                cursor_prefix(true)
            } else {
                "    "
            }
        } else {
            ""
        };

        let display = format!("{}{}", prefix, line);
        f.render_widget(
            Paragraph::new(display).style(highlight_style(selected)),
            Rect::new(inner.x, y, inner.width, 1),
        );
    }

    let hint = " Esc: Close   Enter: Select   ↑↓: Navigate ";
    let hint_y = area.height.saturating_sub(1);
    f.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::DarkGray).bg(OUTPUT_BG)),
        Rect::new(0, hint_y, area.width, 1),
    );
}

fn render_provider_page(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" Select Provider ")
        .title_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(OUTPUT_BG).fg(OUTPUT_FG));
    f.render_widget(block, area);

    let inner = area.inner(Margin {
        vertical: 2,
        horizontal: 4,
    });
    let cursor = app.settings_cursor;
    let current = app.settings_state.provider;

    let providers = [
        (ProviderType::Ollama, "Ollama — Local LLMs via Ollama"),
        (ProviderType::OpenAI, "OpenAI — GPT models via API"),
        (ProviderType::Anthropic, "Anthropic — Claude models via API"),
        (
            ProviderType::Gemini,
            "Gemini — Google Gemini models via API",
        ),
        (
            ProviderType::OpenRouter,
            "OpenRouter — Multi-provider model router",
        ),
        (
            ProviderType::OpenAICompatible,
            "OpenAI Compatible — Custom endpoint",
        ),
    ];

    for (i, (p, label)) in providers.iter().enumerate() {
        let y = inner.y + i as u16;
        if y >= inner.y + inner.height {
            break;
        }
        let selected_mark = if *p == current { " ✓" } else { "" };
        let prefix = cursor_prefix(cursor == i);
        let display = format!("{}{}{}", prefix, label, selected_mark);
        f.render_widget(
            Paragraph::new(display).style(highlight_style(cursor == i)),
            Rect::new(inner.x, y, inner.width, 1),
        );
    }

    let hint = " Esc: Back   Enter: Select   ↑↓: Navigate ";
    let hint_y = area.height.saturating_sub(1);
    f.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::DarkGray).bg(OUTPUT_BG)),
        Rect::new(0, hint_y, area.width, 1),
    );
}

fn render_model_page(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" Select Model ")
        .title_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(OUTPUT_BG).fg(OUTPUT_FG));
    f.render_widget(block, area);

    let inner = area.inner(Margin {
        vertical: 2,
        horizontal: 4,
    });
    let cursor = app.settings_cursor;
    let models = &app.settings_state.available_models;
    let current = &app.settings_state.model;

    if models.is_empty() {
        f.render_widget(
            Paragraph::new("No models available. Try changing provider first.")
                .style(Style::default().fg(OUTPUT_FG)),
            area.inner(Margin {
                vertical: 3,
                horizontal: 4,
            }),
        );
    } else {
        for (i, model) in models.iter().enumerate() {
            let y = inner.y + i as u16;
            if y >= inner.y + inner.height {
                break;
            }
            let selected_mark = if model == current { " ✓" } else { "" };
            let prefix = cursor_prefix(cursor == i);
            let display = format!("{}{}{}", prefix, model, selected_mark);
            f.render_widget(
                Paragraph::new(display).style(highlight_style(cursor == i)),
                Rect::new(inner.x, y, inner.width, 1),
            );
        }
    }

    let hint = " Esc: Back   Enter: Select   ↑↓: Navigate ";
    let hint_y = area.height.saturating_sub(1);
    f.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::DarkGray).bg(OUTPUT_BG)),
        Rect::new(0, hint_y, area.width, 1),
    );
}

fn render_baseurl_page(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" Edit Base URL ")
        .title_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(OUTPUT_BG).fg(OUTPUT_FG));
    f.render_widget(block, area);

    let inner = area.inner(Margin {
        vertical: 3,
        horizontal: 4,
    });
    let url = &app.settings_state.base_url;

    f.render_widget(
        Paragraph::new("Base URL:").style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );

    let input_y = inner.y + 2;
    let field_width = inner.width.min(80);
    f.render_widget(
        Paragraph::new(format!(" {}", url))
            .style(Style::default().fg(OUTPUT_FG).bg(Color::Rgb(30, 30, 30))),
        Rect::new(inner.x, input_y, field_width, 1),
    );
    let cursor_x = inner
        .x
        .saturating_add(1)
        .saturating_add(url.chars().count() as u16);
    let max_x = inner.x.saturating_add(field_width.saturating_sub(1));
    f.set_cursor_position(Position {
        x: cursor_x.min(max_x),
        y: input_y,
    });

    let hint = " Esc: Back   Enter: Confirm   Type to edit ";
    let hint_y = area.height.saturating_sub(1);
    f.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::DarkGray).bg(OUTPUT_BG)),
        Rect::new(0, hint_y, area.width, 1),
    );
}

fn render_apikey_page(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" Edit API Key ")
        .title_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(OUTPUT_BG).fg(OUTPUT_FG));
    f.render_widget(block, area);

    let inner = area.inner(Margin {
        vertical: 3,
        horizontal: 4,
    });
    let key = &app.settings_state.api_key;
    let n = key.chars().count();

    let display = if key.is_empty() {
        "(empty — paste your key with Ctrl+Shift+V)".to_string()
    } else {
        let bullets: String = "•".repeat(n.min(48));
        format!("{bullets}  ({n} chars)")
    };

    f.render_widget(
        Paragraph::new("API Key:").style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );

    f.render_widget(
        Paragraph::new("Gemini keys start with AIza… and are ~39 characters. Paste recommended.")
            .style(Style::default().fg(Color::DarkGray)),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    let input_y = inner.y + 3;
    let field_width = inner.width.min(80);
    f.render_widget(
        Paragraph::new(format!(" {}", display))
            .style(Style::default().fg(OUTPUT_FG).bg(Color::Rgb(30, 30, 30))),
        Rect::new(inner.x, input_y, field_width, 1),
    );
    let cursor_x = inner
        .x
        .saturating_add(1)
        .saturating_add(display.chars().count() as u16);
    let max_x = inner.x.saturating_add(field_width.saturating_sub(1));
    f.set_cursor_position(Position {
        x: cursor_x.min(max_x),
        y: input_y,
    });

    let hint = " Esc: Back   Enter: Confirm   Ctrl+Shift+V / Ctrl+V: Paste   Middle-click: Paste   Ctrl+U: Clear ";
    let hint_y = area.height.saturating_sub(1);
    f.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::DarkGray).bg(OUTPUT_BG)),
        Rect::new(0, hint_y, area.width, 1),
    );
}

fn render_enable_page(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" Enable AI ")
        .title_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(OUTPUT_BG).fg(OUTPUT_FG));
    f.render_widget(block, area);

    let inner = area.inner(Margin {
        vertical: 3,
        horizontal: 4,
    });
    let cursor = app.settings_cursor;
    let enabled = app.settings_state.enabled;

    let options = ["Yes — AI features enabled", "No  — AI features disabled"];
    let values = [true, false];

    for (i, label) in options.iter().enumerate() {
        let y = inner.y + 2 + i as u16;
        if y >= inner.y + inner.height {
            break;
        }
        let selected_mark = if values[i] == enabled { " ✓" } else { "" };
        let prefix = cursor_prefix(cursor == i);
        let display = format!("{}{}{}", prefix, label, selected_mark);
        f.render_widget(
            Paragraph::new(display).style(highlight_style(cursor == i)),
            Rect::new(inner.x, y, inner.width, 1),
        );
    }

    let hint = " Esc: Back   Enter: Toggle ";
    let hint_y = area.height.saturating_sub(1);
    f.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::DarkGray).bg(OUTPUT_BG)),
        Rect::new(0, hint_y, area.width, 1),
    );
}
