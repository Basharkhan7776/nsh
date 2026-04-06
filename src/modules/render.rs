// UI rendering for terminal shell

use super::commands::shorten_cwd;
use super::config::*;
use super::state::{App, EntryType};
use crate::ai::ProviderType;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};

const DIALOG_WIDTH: u16 = 50;

pub fn render(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &App,
) -> std::io::Result<()> {
    terminal.draw(|f| {
        if app.show_settings {
            render_settings_dialog(f, app);
        } else {
            render_shell(f, app);
        }
    })?;
    Ok(())
}

fn render_shell(f: &mut ratatui::Frame, app: &App) {
    // Define styles for each UI element
    let output_bg = Style::default().bg(OUTPUT_BG);
    let output_fg = Style::default().fg(OUTPUT_FG).bg(OUTPUT_BG);
    let cwd_style = Style::default().fg(CWD_FG).bg(OUTPUT_BG);
    let cmd_style = Style::default().fg(COMMAND_FG).bg(OUTPUT_BG);
    let input_style = Style::default().fg(INPUT_PROMPT_FG).bg(INPUT_BG);
    let suggestion_style = Style::default().fg(SUGGESTION_INDICATOR_FG).bg(INPUT_BG);
    let system_style = Style::default().fg(SYSTEM_FG).bg(OUTPUT_BG);

    // Split terminal into output and input areas
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(f.area());

    let list_area = chunks[0]; // History/output area
    let input_area = chunks[1]; // Input line area

    // Calculate visible range based on scroll offset
    let visible_height = list_area.height as usize;
    let content_height = app.entries.iter().map(|e| e.content.len()).sum::<usize>();
    let start_line = app.scroll_offset;
    let end_line = (start_line + visible_height).min(content_height);

    // Build list of visible items
    let mut current_line = 0;
    let mut items: Vec<ListItem> = Vec::new();

    for entry in &app.entries {
        let entry_height = entry.content.len();
        let entry_end = current_line + entry_height;

        // Skip entries above visible range
        if entry_end <= start_line {
            current_line = entry_end;
            continue;
        }

        // Calculate visible portion within entry
        let skip = if current_line < start_line {
            start_line - current_line
        } else {
            0
        };
        let show_from = current_line + skip;
        let show_to = entry_end.min(end_line);

        // Render each visible line
        for i in show_from..show_to {
            let line_idx = i - current_line;
            if let Some(line) = entry.content.get(line_idx) {
                match entry.entry_type {
                    EntryType::Command => {
                        if let Some(cmd) = entry.content.first() {
                            if i == current_line {
                                // Format: <cwd> $ <command>
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

    // Render output area
    let list = List::new(items).style(output_bg);
    f.render_widget(list, list_area);

    // Render input line with cursor
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

    // Render suggestion popup if visible
    if app.show_suggestions && !app.current_suggestions.is_empty() {
        let visible = app.visible_suggestions();
        let has_more = app.has_more_suggestions();

        let display_height = if has_more {
            MAX_VISIBLE_SUGGESTIONS + 1
        } else {
            visible.len().min(MAX_VISIBLE_SUGGESTIONS)
        };
        let display_height = display_height as u16;

        // Build suggestion list items
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

        // Add "more" indicator if additional suggestions exist
        if has_more {
            let more_item = ListItem::new(Line::from(Span::styled(
                "...",
                Style::default().fg(SUGGESTION_INDICATOR_FG),
            )));
            suggestions_items.push(more_item);
        }

        // Render suggestion popup
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

fn render_settings_dialog(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();

    // Full screen overlay with semi-transparent background
    let overlay = Block::default().style(Style::default().bg(Color::Rgb(0, 0, 0).into()));
    f.render_widget(overlay, area);

    // Settings panel - centered, 60% width
    let panel_width = (area.width as f32 * 0.6) as u16;
    let panel_height = (area.height as f32 * 0.7) as u16;
    let x = (area.width - panel_width) / 2;
    let y = (area.height - panel_height) / 2;

    let panel_rect = Rect::new(x, y, x + panel_width, y + panel_height);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title("⚙ AI Settings")
        .title_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(OUTPUT_BG).fg(OUTPUT_FG));

    f.render_widget(block, panel_rect);

    let inner = panel_rect.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    let state = &app.settings_state;
    let cursor = app.settings_cursor;

    let provider_str = match state.provider {
        ProviderType::Ollama => "Ollama",
        ProviderType::OpenAI => "OpenAI",
        ProviderType::Anthropic => "Anthropic",
        ProviderType::OpenAICompatible => "OpenAI Compatible",
    };

    let enable_str = if state.enabled { "Yes" } else { "No" };
    let api_key_str = if state.api_key_original.is_empty() {
        "(empty)"
    } else {
        "••••••••••••••"
    };

    // Build lines as strings
    let lines: Vec<String> = vec![
        format!(
            "{}Provider:  {}",
            if cursor == 0 { "▶ " } else { "  " },
            provider_str
        ),
        format!(
            "{}Model:     {}",
            if cursor == 1 { "▶ " } else { "  " },
            state.model
        ),
        format!(
            "{}Base URL:  {}",
            if cursor == 2 { "▶ " } else { "  " },
            state.base_url
        ),
        format!(
            "{}API Key:   {}",
            if cursor == 3 { "▶ " } else { "  " },
            api_key_str
        ),
        format!(
            "{}Enable:    {}",
            if cursor == 4 { "▶ " } else { "  " },
            enable_str
        ),
        String::new(),
        format!(
            "{}[ Save ]{}  [ Cancel ]",
            if cursor == 5 { "▶ " } else { "  " },
            if cursor == 6 { " " } else { "  " }
        ),
    ];

    // Render lines - selected items in green, others in white
    for (i, line) in lines.iter().enumerate() {
        let y = inner.y + i as u16;
        if y < inner.y + inner.height {
            let style = if i < 5 && cursor == i as usize {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else if i == 6 && cursor == 5 {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else if i == 6 && cursor == 6 {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(OUTPUT_FG)
            };
            f.render_widget(
                Paragraph::new(line.as_str()).style(style),
                Rect::new(inner.x, y, inner.x + inner.width, y + 1),
            );
        }
    }

    // Dropdown rendering
    if state.show_provider_dropdown {
        let dropdown_height = ProviderType::count() as u16;
        let dropdown_rect = Rect::new(
            inner.x + 12,
            inner.y + 1,
            inner.x + 35,
            inner.y + 1 + dropdown_height,
        );

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green))
            .style(Style::default().bg(Color::Black).fg(Color::White));
        f.render_widget(block, dropdown_rect);

        for (i, provider) in [
            ProviderType::Ollama,
            ProviderType::OpenAI,
            ProviderType::Anthropic,
            ProviderType::OpenAICompatible,
        ]
        .iter()
        .enumerate()
        {
            let name = match provider {
                ProviderType::Ollama => "Ollama",
                ProviderType::OpenAI => "OpenAI",
                ProviderType::Anthropic => "Anthropic",
                ProviderType::OpenAICompatible => "OpenAI Compatible",
            };
            let is_selected = i == state.dropdown_cursor;
            let style = if is_selected {
                Style::default().bg(Color::Green).fg(Color::Black)
            } else {
                Style::default().fg(Color::White)
            };
            f.render_widget(
                Paragraph::new(name).style(style),
                Rect::new(
                    dropdown_rect.x + 1,
                    dropdown_rect.y + i as u16,
                    dropdown_rect.right() - 1,
                    dropdown_rect.y + i as u16 + 1,
                ),
            );
        }
    }

    if state.show_model_dropdown && !state.available_models.is_empty() {
        let dropdown_height = state.available_models.len().min(6) as u16;
        let dropdown_rect = Rect::new(
            inner.x + 12,
            inner.y + 2,
            inner.x + 45,
            inner.y + 2 + dropdown_height,
        );

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green))
            .style(Style::default().bg(Color::Black).fg(Color::White));
        f.render_widget(block, dropdown_rect);

        for (i, model) in state.available_models.iter().enumerate() {
            if i >= 6 {
                break;
            }
            let is_selected = i == state.dropdown_cursor;
            let style = if is_selected {
                Style::default().bg(Color::Green).fg(Color::Black)
            } else {
                Style::default().fg(Color::White)
            };
            f.render_widget(
                Paragraph::new(model.as_str()).style(style),
                Rect::new(
                    dropdown_rect.x + 1,
                    dropdown_rect.y + i as u16,
                    dropdown_rect.right() - 1,
                    dropdown_rect.y + i as u16 + 1,
                ),
            );
        }
    }
}
