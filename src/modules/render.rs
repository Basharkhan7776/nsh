// UI rendering for terminal shell

use super::commands::shorten_cwd;
use super::config::*;
use super::state::{App, EntryType, SettingsPage};
use crate::ai::ProviderType;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};

pub fn render(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &App,
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

fn render_shell(f: &mut ratatui::Frame, app: &App) {
    let output_bg = Style::default().bg(OUTPUT_BG);
    let output_fg = Style::default().fg(OUTPUT_FG).bg(OUTPUT_BG);
    let cwd_style = Style::default().fg(CWD_FG).bg(OUTPUT_BG);
    let cmd_style = Style::default().fg(COMMAND_FG).bg(OUTPUT_BG);
    let input_style = Style::default().fg(INPUT_PROMPT_FG).bg(INPUT_BG);
    let suggestion_style = Style::default().fg(SUGGESTION_INDICATOR_FG).bg(INPUT_BG);
    let system_style = Style::default().fg(SYSTEM_FG).bg(OUTPUT_BG);

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
                                let cwd_display = shorten_cwd(&entry.cwd);
                                let cmd_display = format!("$ {}", cmd);

                                let line = Line::from(vec![
                                    Span::styled(cwd_display, cwd_style),
                                    Span::styled(cmd_display, cmd_style),
                                ]);
                                items.push(ListItem::new(line));
                            }
                        }
                    }
                    EntryType::Output => {
                        items.push(ListItem::new(Line::from(Span::styled(line, output_fg))));
                    }
                    EntryType::System => {
                        items.push(ListItem::new(Line::from(Span::styled(line, system_style))));
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

    let input_with_cursor = if app.current_input.is_empty() {
        format!("{}|", PROMPT_TEXT)
    } else if app.cursor_position == 0 {
        format!("{}|{}", PROMPT_TEXT, app.current_input)
    } else if app.cursor_position >= app.current_input.len() {
        format!("{}{}|", PROMPT_TEXT, app.current_input)
    } else {
        let before_cursor = &app.current_input[..app.cursor_position];
        let after_cursor = &app.current_input[app.cursor_position..];
        format!("{}{}|{}", PROMPT_TEXT, before_cursor, after_cursor)
    };
    let input_widget = Paragraph::new(input_with_cursor.as_str()).style(input_style);
    f.render_widget(input_widget, input_area);

    if app.show_suggestions && !app.current_suggestions.is_empty() {
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
            y: input_area.y.saturating_sub(display_height as u16),
            width: 40.min(input_area.width - 2),
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
        .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(OUTPUT_BG).fg(OUTPUT_FG));
    f.render_widget(block, area);

    let inner = area.inner(Margin { vertical: 2, horizontal: 3 });
    let state = &app.settings_state;
    let cursor = app.settings_cursor;

    let provider_str = match state.provider {
        ProviderType::Ollama => "Ollama",
        ProviderType::OpenAI => "OpenAI",
        ProviderType::Anthropic => "Anthropic",
        ProviderType::OpenAICompatible => "OpenAI Compatible",
    };

    let api_key_str = if state.api_key_original.is_empty() {
        "(empty)"
    } else {
        "••••••••••••••"
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
        Paragraph::new(hint)
            .style(Style::default().fg(Color::DarkGray).bg(OUTPUT_BG)),
        Rect::new(0, hint_y, area.width, 1),
    );
}

fn render_provider_page(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" Select Provider ")
        .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(OUTPUT_BG).fg(OUTPUT_FG));
    f.render_widget(block, area);

    let inner = area.inner(Margin { vertical: 2, horizontal: 4 });
    let cursor = app.settings_cursor;
    let current = app.settings_state.provider;

    let providers = [
        (ProviderType::Ollama, "Ollama — Local LLMs via Ollama"),
        (ProviderType::OpenAI, "OpenAI — GPT models via API"),
        (ProviderType::Anthropic, "Anthropic — Claude models via API"),
        (ProviderType::OpenAICompatible, "OpenAI Compatible — Custom endpoint"),
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
        Paragraph::new(hint)
            .style(Style::default().fg(Color::DarkGray).bg(OUTPUT_BG)),
        Rect::new(0, hint_y, area.width, 1),
    );
}

fn render_model_page(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" Select Model ")
        .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(OUTPUT_BG).fg(OUTPUT_FG));
    f.render_widget(block, area);

    let inner = area.inner(Margin { vertical: 2, horizontal: 4 });
    let cursor = app.settings_cursor;
    let models = &app.settings_state.available_models;
    let current = &app.settings_state.model;

    if models.is_empty() {
        f.render_widget(
            Paragraph::new("No models available. Try changing provider first.")
                .style(Style::default().fg(OUTPUT_FG)),
            area.inner(Margin { vertical: 3, horizontal: 4 }),
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
        Paragraph::new(hint)
            .style(Style::default().fg(Color::DarkGray).bg(OUTPUT_BG)),
        Rect::new(0, hint_y, area.width, 1),
    );
}

fn render_baseurl_page(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" Edit Base URL ")
        .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(OUTPUT_BG).fg(OUTPUT_FG));
    f.render_widget(block, area);

    let inner = area.inner(Margin { vertical: 3, horizontal: 4 });
    let url = &app.settings_state.base_url;

    f.render_widget(
        Paragraph::new("Base URL:")
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );

    let input_y = inner.y + 2;
    let cursor_x = inner.x + 1 + url.len() as u16;
    f.render_widget(
        Paragraph::new(format!(" {}", url))
            .style(Style::default().fg(OUTPUT_FG).bg(Color::Rgb(30, 30, 30))),
        Rect::new(inner.x, input_y, inner.width.min(80), 3),
    );

    let display_with_cursor = format!(" {}|", url);
    if cursor_x < inner.x + inner.width.min(80) {
        f.render_widget(
            Paragraph::new(display_with_cursor)
                .style(Style::default().fg(Color::Green).bg(Color::Rgb(30, 30, 30))),
            Rect::new(inner.x, input_y, inner.width.min(80), 1),
        );
    }

    let hint = " Esc: Back   Enter: Confirm   Type to edit ";
    let hint_y = area.height.saturating_sub(1);
    f.render_widget(
        Paragraph::new(hint)
            .style(Style::default().fg(Color::DarkGray).bg(OUTPUT_BG)),
        Rect::new(0, hint_y, area.width, 1),
    );
}

fn render_apikey_page(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" Edit API Key ")
        .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(OUTPUT_BG).fg(OUTPUT_FG));
    f.render_widget(block, area);

    let inner = area.inner(Margin { vertical: 3, horizontal: 4 });
    let key = &app.settings_state.api_key;

    let display = if key.is_empty() {
        "(empty)"
    } else {
        "••••••••••••••"
    };

    f.render_widget(
        Paragraph::new("API Key:")
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );

    let input_y = inner.y + 2;
    f.render_widget(
        Paragraph::new(format!(" {}", display))
            .style(Style::default().fg(OUTPUT_FG).bg(Color::Rgb(30, 30, 30))),
        Rect::new(inner.x, input_y, inner.width.min(80), 1),
    );

    let hint = " Esc: Back   Enter: Confirm   Type to edit (key is hidden) ";
    let hint_y = area.height.saturating_sub(1);
    f.render_widget(
        Paragraph::new(hint)
            .style(Style::default().fg(Color::DarkGray).bg(OUTPUT_BG)),
        Rect::new(0, hint_y, area.width, 1),
    );
}

fn render_enable_page(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" Enable AI ")
        .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(OUTPUT_BG).fg(OUTPUT_FG));
    f.render_widget(block, area);

    let inner = area.inner(Margin { vertical: 3, horizontal: 4 });
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
        Paragraph::new(hint)
            .style(Style::default().fg(Color::DarkGray).bg(OUTPUT_BG)),
        Rect::new(0, hint_y, area.width, 1),
    );
}
