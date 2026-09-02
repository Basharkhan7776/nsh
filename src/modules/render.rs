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
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
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

        let max_item_len = visible.iter().map(|s| s.chars().count()).max().unwrap_or(20);
        let box_width = (max_item_len as u16 + 4).clamp(40, input_area.width.saturating_sub(4));
        let suggestions_list = List::new(suggestions_items).style(suggestion_style);
        let suggestions_area = Rect {
            x: 2,
            y: input_area.y.saturating_sub(display_height),
            width: box_width,
            height: display_height,
        };
        f.render_widget(suggestions_list, suggestions_area);
    }
}

// ── Settings Modal UI (Monochrome Terminal UI) ────────────────────────────────

pub const MODAL_BORDER: Color = Color::White;
pub const MODAL_TITLE: Color = Color::White;
pub const MODAL_TEXT: Color = Color::White;
pub const MODAL_MUTED: Color = Color::DarkGray;
pub const MODAL_DIVIDER: Color = Color::DarkGray;
pub const MODAL_SELECTED_BG: Color = Color::Rgb(40, 40, 40);

/// Compute the centered modal rectangle inside the terminal viewport.
pub fn compute_settings_modal_area(area: Rect) -> Rect {
    let modal_w = ((area.width as f32 * 0.78) as u16).clamp(52, 92).min(area.width);
    let modal_h = ((area.height as f32 * 0.80) as u16).clamp(18, 28).min(area.height);
    let x = area.x + (area.width.saturating_sub(modal_w)) / 2;
    let y = area.y + (area.height.saturating_sub(modal_h)) / 2;
    Rect::new(x, y, modal_w, modal_h)
}

fn shorten_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        format!("{truncated}...")
    }
}

fn render_settings(f: &mut ratatui::Frame, app: &App) {
    let modal_area = compute_settings_modal_area(f.area());

    // Clear cells underneath so terminal contents don't bleed through
    f.render_widget(Clear, modal_area);

    // Modal frame box (white borders, no background like terminal UI)
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MODAL_BORDER));
    f.render_widget(block, modal_area);

    let page = app.current_settings_page();

    // Top border title with breadcrumb (no emojis, regular weight):
    let breadcrumb = match page {
        SettingsPage::Home => " Settings ".to_string(),
        SettingsPage::Provider => " Settings > Provider ".to_string(),
        SettingsPage::Model => " Settings > Model ".to_string(),
        SettingsPage::BaseUrl => " Settings > Base URL ".to_string(),
        SettingsPage::ApiKey => " Settings > API Key ".to_string(),
        SettingsPage::Enable => " Settings > Enable AI ".to_string(),
    };
    let title_line = Line::from(vec![
        Span::styled(
            breadcrumb,
            Style::default().fg(MODAL_TITLE),
        ),
    ]);
    f.render_widget(
        Paragraph::new(title_line),
        Rect::new(modal_area.x + 2, modal_area.y, modal_area.width.saturating_sub(14), 1),
    );

    // Top-right close button (no emojis)
    let close_hint = "[Esc X]";
    f.render_widget(
        Paragraph::new(close_hint).style(Style::default().fg(MODAL_MUTED)),
        Rect::new(modal_area.x + modal_area.width.saturating_sub(9), modal_area.y, 7, 1),
    );

    // Bottom border shortcuts footer (monochrome, no background, regular weight)
    let footer_line = Line::from(vec![
        Span::styled(" Up/Down/jk ", Style::default().fg(Color::White)),
        Span::styled("navigate  ", Style::default().fg(MODAL_MUTED)),
        Span::styled("Enter ", Style::default().fg(Color::White)),
        Span::styled("select  ", Style::default().fg(MODAL_MUTED)),
        Span::styled("Space ", Style::default().fg(Color::White)),
        Span::styled("toggle  ", Style::default().fg(MODAL_MUTED)),
        Span::styled("/ ", Style::default().fg(Color::White)),
        Span::styled("filter  ", Style::default().fg(MODAL_MUTED)),
        Span::styled("Esc ", Style::default().fg(Color::White)),
        Span::styled("close  ", Style::default().fg(MODAL_MUTED)),
        Span::styled("Ctrl+S ", Style::default().fg(Color::White)),
        Span::styled("save", Style::default().fg(MODAL_MUTED)),
    ]);
    let footer_y = modal_area.y + modal_area.height.saturating_sub(1);
    f.render_widget(
        Paragraph::new(footer_line).centered(),
        Rect::new(modal_area.x + 1, footer_y, modal_area.width.saturating_sub(2), 1),
    );

    let inner = modal_area.inner(Margin {
        vertical: 1,
        horizontal: 2,
    });

    match page {
        SettingsPage::Home => render_home_page(f, app, inner),
        SettingsPage::Provider => render_provider_page(f, app, inner),
        SettingsPage::Model => render_model_page(f, app, inner),
        SettingsPage::BaseUrl => render_baseurl_page(f, app, inner),
        SettingsPage::ApiKey => render_apikey_page(f, app, inner),
        SettingsPage::Enable => render_enable_page(f, app, inner),
    }
}

fn render_home_page(f: &mut ratatui::Frame, app: &App, inner: Rect) {
    let state = &app.settings_state;
    let cursor = app.settings_cursor;

    // Row 0: Search / Filter bar (no emojis, terminal UI)
    let filter_rect = Rect::new(inner.x, inner.y, inner.width, 1);
    let (filter_text, filter_style) = if app.settings_filter_active {
        (
            format!(" Filter: {}_", app.settings_filter),
            Style::default().fg(Color::White).bg(MODAL_SELECTED_BG),
        )
    } else if !app.settings_filter.is_empty() {
        (
            format!(" Filter: {} (press / to edit, Esc to clear)", app.settings_filter),
            Style::default().fg(Color::White),
        )
    } else {
        (
            " Filter: press / to search settings...".to_string(),
            Style::default().fg(MODAL_MUTED),
        )
    };
    f.render_widget(Paragraph::new(filter_text).style(filter_style), filter_rect);

    // Row 1: Divider
    let div_rect = Rect::new(inner.x, inner.y + 1, inner.width, 1);
    let div_str: String = "─".repeat(inner.width as usize);
    f.render_widget(Paragraph::new(div_str).style(Style::default().fg(MODAL_DIVIDER)), div_rect);

    let provider_str = state.provider.display_name();
    let key_len = state.api_key_original.trim().chars().count();
    let api_key_display = if key_len == 0 {
        "(not set) >".to_string()
    } else {
        format!("******** ({key_len} chars) >")
    };
    let enable_badge = if state.enabled {
        "[ ON  ]  (Space to toggle)"
    } else {
        "[ OFF ]  (Space to toggle)"
    };

    struct RowDef {
        field_idx: usize,
        category: &'static str,
        label: &'static str,
        value: String,
        desc: &'static str,
    }

    let all_rows = [
        RowDef {
            field_idx: 0,
            category: "AI ENGINE",
            label: "Provider",
            value: format!("{} >", provider_str),
            desc: "Inference backend: Ollama, OpenAI, Anthropic, Gemini, OpenRouter",
        },
        RowDef {
            field_idx: 1,
            category: "AI ENGINE",
            label: "Model",
            value: format!("{} >", state.model),
            desc: "Active model for reasoning and tool generation",
        },
        RowDef {
            field_idx: 2,
            category: "CREDENTIALS & ENDPOINTS",
            label: "Base URL",
            value: format!("{} >", shorten_str(&state.base_url, 32)),
            desc: "HTTP endpoint for OpenAI-compatible completions API",
        },
        RowDef {
            field_idx: 3,
            category: "CREDENTIALS & ENDPOINTS",
            label: "API Key",
            value: api_key_display,
            desc: "Secret key stored in ~/.config/nsh/config.json",
        },
        RowDef {
            field_idx: 4,
            category: "SYSTEM",
            label: "Enable AI",
            value: enable_badge.to_string(),
            desc: "Toggle AI commands (ask, do, plan, build) in nsh",
        },
    ];

    let q = app.settings_filter.to_lowercase();
    let matches_filter = |label: &str, desc: &str, cat: &str| -> bool {
        if q.is_empty() {
            true
        } else {
            label.to_lowercase().contains(&q)
                || desc.to_lowercase().contains(&q)
                || cat.to_lowercase().contains(&q)
        }
    };

    let mut cur_y = inner.y + 2;
    let mut last_category = "";

    for row in &all_rows {
        if !matches_filter(row.label, row.desc, row.category) {
            continue;
        }

        if cur_y >= inner.y + inner.height.saturating_sub(3) {
            break;
        }

        // Category header (muted gray, normal weight)
        if row.category != last_category {
            last_category = row.category;
            let cat_title = format!("── {} ", row.category);
            let cat_w = cat_title.chars().count();
            let cat_sep: String = "─".repeat((inner.width as usize).saturating_sub(cat_w));
            let cat_line = Line::from(vec![
                Span::styled(cat_title, Style::default().fg(MODAL_MUTED)),
                Span::styled(cat_sep, Style::default().fg(MODAL_DIVIDER)),
            ]);
            f.render_widget(Paragraph::new(cat_line), Rect::new(inner.x, cur_y, inner.width, 1));
            cur_y += 1;
        }

        if cur_y >= inner.y + inner.height.saturating_sub(3) {
            break;
        }

        let is_selected = cursor == row.field_idx;

        // Terminal UI row selection: gray background when selected
        if is_selected {
            f.render_widget(
                Block::default().style(Style::default().bg(MODAL_SELECTED_BG)),
                Rect::new(inner.x, cur_y, inner.width, 1),
            );
        }

        let row_style = if is_selected {
            Style::default().fg(Color::White).bg(MODAL_SELECTED_BG)
        } else {
            Style::default().fg(Color::White)
        };

        // Left label with pointer (regular weight)
        let prefix = if is_selected { "> " } else { "  " };
        let label_line = Line::from(vec![
            Span::styled(prefix, row_style),
            Span::styled(row.label, row_style),
        ]);
        f.render_widget(Paragraph::new(label_line), Rect::new(inner.x + 1, cur_y, inner.width / 2, 1));

        // Right value (regular weight)
        let val_style = if is_selected {
            row_style
        } else if row.field_idx == 4 {
            if state.enabled {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(MODAL_MUTED)
            }
        } else {
            Style::default().fg(Color::White)
        };
        f.render_widget(
            Paragraph::new(row.value.as_str()).style(val_style).right_aligned(),
            Rect::new(inner.x, cur_y, inner.width.saturating_sub(2), 1),
        );

        cur_y += 1;
    }

    // Selected description helper (clean text, regular weight)
    let selected_desc = match cursor {
        0 => "Inference backend: Ollama, OpenAI, Anthropic, Gemini, OpenRouter",
        1 => "Active model for reasoning and terminal tool execution",
        2 => "HTTP endpoint URL (OpenAI-compatible /v1 format)",
        3 => "Secret key stored securely in ~/.config/nsh/config.json",
        4 => "Toggle ask, do, plan, build terminal verbs (Space to toggle)",
        5 => "Save changes to config file and apply immediately",
        6 => "Discard in-memory edits and return to shell",
        _ => "",
    };

    let desc_y = inner.y + inner.height.saturating_sub(3);
    f.render_widget(
        Paragraph::new(format!("  Note: {}", selected_desc))
            .style(Style::default().fg(MODAL_MUTED)),
        Rect::new(inner.x, desc_y, inner.width, 1),
    );

    // Actions divider and buttons
    let act_div_y = inner.y + inner.height.saturating_sub(2);
    let act_title = "── ACTIONS ";
    let act_sep: String = "─".repeat((inner.width as usize).saturating_sub(act_title.len()));
    let act_line = Line::from(vec![
        Span::styled(act_title, Style::default().fg(MODAL_MUTED)),
        Span::styled(act_sep, Style::default().fg(MODAL_DIVIDER)),
    ]);
    f.render_widget(Paragraph::new(act_line), Rect::new(inner.x, act_div_y, inner.width, 1));

    let btns_y = inner.y + inner.height.saturating_sub(1);
    let is_save = cursor == 5;
    let is_cancel = cursor == 6;

    let save_style = if is_save {
        Style::default().fg(Color::White).bg(MODAL_SELECTED_BG)
    } else {
        Style::default().fg(Color::White)
    };

    let cancel_style = if is_cancel {
        Style::default().fg(Color::White).bg(MODAL_SELECTED_BG)
    } else {
        Style::default().fg(MODAL_MUTED)
    };

    let buttons_line = Line::from(vec![
        Span::raw("  "),
        Span::styled(" [ Save & Apply (Ctrl+S) ] ", save_style),
        Span::raw("   "),
        Span::styled(" [ Cancel (Esc) ] ", cancel_style),
    ]);
    f.render_widget(Paragraph::new(buttons_line), Rect::new(inner.x, btns_y, inner.width, 1));
}

fn render_provider_page(f: &mut ratatui::Frame, app: &App, inner: Rect) {
    let cursor = app.settings_cursor;
    let current = app.settings_state.provider;

    // Header (regular weight)
    let header_line = Line::from(vec![
        Span::styled("Select AI Provider", Style::default().fg(MODAL_TITLE)),
    ]);
    let sub_line = Span::styled(
        "Choose the LLM inference provider or gateway backend.",
        Style::default().fg(MODAL_MUTED),
    );
    f.render_widget(Paragraph::new(header_line), Rect::new(inner.x, inner.y, inner.width, 1));
    f.render_widget(Paragraph::new(sub_line), Rect::new(inner.x, inner.y + 1, inner.width, 1));

    // Divider
    let div_str: String = "─".repeat(inner.width as usize);
    f.render_widget(Paragraph::new(div_str).style(Style::default().fg(MODAL_DIVIDER)), Rect::new(inner.x, inner.y + 2, inner.width, 1));

    let providers = [
        (ProviderType::Ollama, "Ollama", "Local model server (no API key required)"),
        (ProviderType::OpenAI, "OpenAI", "GPT-4o, o1, o3 models via official API"),
        (ProviderType::Anthropic, "Anthropic", "Claude 3.5 Sonnet, Claude 3 Opus via API"),
        (ProviderType::Gemini, "Gemini", "Google Gemini 2.5 Flash, Pro via API"),
        (ProviderType::OpenRouter, "OpenRouter", "Unified router for hundreds of models"),
        (ProviderType::OpenAICompatible, "Custom / Compatible", "Self-hosted vLLM, LM Studio, or proxy endpoint"),
    ];

    let start_y = inner.y + 3;
    for (i, (p, label, desc)) in providers.iter().enumerate() {
        let y = start_y + i as u16 * 2;
        if y + 1 >= inner.y + inner.height {
            break;
        }

        let is_selected = cursor == i;
        let is_current = *p == current;

        let radio_str = if is_current { "(*) " } else { "( ) " };

        if is_selected {
            f.render_widget(
                Block::default().style(Style::default().bg(MODAL_SELECTED_BG)),
                Rect::new(inner.x, y, inner.width, 2),
            );
        }

        let row_style = if is_selected {
            Style::default().fg(Color::White).bg(MODAL_SELECTED_BG)
        } else {
            Style::default().fg(Color::White)
        };

        let active_tag = if is_current { " (current)" } else { "" };
        let name_line = Line::from(vec![
            Span::raw("  "),
            Span::styled(radio_str, row_style),
            Span::styled(*label, row_style),
            Span::styled(active_tag, if is_selected { row_style } else { Style::default().fg(MODAL_MUTED) }),
        ]);
        f.render_widget(Paragraph::new(name_line), Rect::new(inner.x, y, inner.width, 1));

        let desc_style = if is_selected {
            Style::default().fg(Color::White).bg(MODAL_SELECTED_BG)
        } else {
            Style::default().fg(MODAL_MUTED)
        };
        let desc_line = Line::from(vec![
            Span::raw("      "),
            Span::styled(*desc, desc_style),
        ]);
        f.render_widget(Paragraph::new(desc_line), Rect::new(inner.x, y + 1, inner.width, 1));
    }
}

fn render_model_page(f: &mut ratatui::Frame, app: &App, inner: Rect) {
    let cursor = app.settings_cursor;
    let models = &app.settings_state.available_models;
    let current = &app.settings_state.model;

    // Header (regular weight)
    let header_line = Line::from(vec![
        Span::styled("Select Model", Style::default().fg(MODAL_TITLE)),
    ]);
    let sub_line = Span::styled(
        "Choose the active model for reasoning and tool generation.",
        Style::default().fg(MODAL_MUTED),
    );
    f.render_widget(Paragraph::new(header_line), Rect::new(inner.x, inner.y, inner.width, 1));
    f.render_widget(Paragraph::new(sub_line), Rect::new(inner.x, inner.y + 1, inner.width, 1));

    // Divider
    let div_str: String = "─".repeat(inner.width as usize);
    f.render_widget(Paragraph::new(div_str).style(Style::default().fg(MODAL_DIVIDER)), Rect::new(inner.x, inner.y + 2, inner.width, 1));

    if models.is_empty() {
        f.render_widget(
            Paragraph::new("No models detected. Check provider connection or customize Base URL in settings.")
                .style(Style::default().fg(MODAL_MUTED)),
            Rect::new(inner.x + 2, inner.y + 4, inner.width.saturating_sub(4), 2),
        );
    } else {
        let start_y = inner.y + 3;
        for (i, model) in models.iter().enumerate() {
            let y = start_y + i as u16;
            if y >= inner.y + inner.height.saturating_sub(1) {
                break;
            }

            let is_selected = cursor == i;
            let is_current = model == current;

            let radio_str = if is_current { "(*) " } else { "( ) " };

            if is_selected {
                f.render_widget(
                    Block::default().style(Style::default().bg(MODAL_SELECTED_BG)),
                    Rect::new(inner.x, y, inner.width, 1),
                );
            }

            let row_style = if is_selected {
                Style::default().fg(Color::White).bg(MODAL_SELECTED_BG)
            } else {
                Style::default().fg(Color::White)
            };

            let active_tag = if is_current { "  (current)" } else { "" };
            let line = Line::from(vec![
                Span::raw("  "),
                Span::styled(radio_str, row_style),
                Span::styled(model.as_str(), row_style),
                Span::styled(active_tag, if is_selected { row_style } else { Style::default().fg(MODAL_MUTED) }),
            ]);
            f.render_widget(Paragraph::new(line), Rect::new(inner.x, y, inner.width, 1));
        }
    }
}

fn render_baseurl_page(f: &mut ratatui::Frame, app: &App, inner: Rect) {
    let url = &app.settings_state.base_url;
    let provider = app.settings_state.provider;

    // Header (regular weight)
    let header_line = Line::from(vec![
        Span::styled("Edit Base URL", Style::default().fg(MODAL_TITLE)),
    ]);
    let sub_line = Span::styled(
        "API endpoint URL for completions and model requests.",
        Style::default().fg(MODAL_MUTED),
    );
    f.render_widget(Paragraph::new(header_line), Rect::new(inner.x, inner.y, inner.width, 1));
    f.render_widget(Paragraph::new(sub_line), Rect::new(inner.x, inner.y + 1, inner.width, 1));

    let div_str: String = "─".repeat(inner.width as usize);
    f.render_widget(Paragraph::new(div_str).style(Style::default().fg(MODAL_DIVIDER)), Rect::new(inner.x, inner.y + 2, inner.width, 1));

    // Input Card Block (white borders, regular weight title)
    let input_box_area = Rect::new(inner.x + 2, inner.y + 4, inner.width.saturating_sub(4), 3);
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MODAL_BORDER))
        .title(" Endpoint URL ")
        .title_style(Style::default().fg(Color::White));
    f.render_widget(input_block, input_box_area);

    let text_line = format!(" {}", url);
    f.render_widget(
        Paragraph::new(text_line).style(Style::default().fg(Color::White)),
        Rect::new(input_box_area.x + 1, input_box_area.y + 1, input_box_area.width.saturating_sub(2), 1),
    );

    // Position cursor
    let cursor_x = input_box_area.x + 2 + url.chars().count() as u16;
    let max_cursor_x = input_box_area.x + input_box_area.width.saturating_sub(2);
    f.set_cursor_position(Position {
        x: cursor_x.min(max_cursor_x),
        y: input_box_area.y + 1,
    });

    // Helper notes
    let note1 = format!("Default for {}: {}", provider.display_name(), provider.default_url());
    f.render_widget(
        Paragraph::new(note1).style(Style::default().fg(Color::White)),
        Rect::new(inner.x + 2, inner.y + 8, inner.width.saturating_sub(4), 1),
    );
    let note2 = "Tips: Ctrl+U clears line | Paste with Ctrl+V or Ctrl+Shift+V | Enter confirms | Esc cancels";
    f.render_widget(
        Paragraph::new(note2).style(Style::default().fg(MODAL_MUTED)),
        Rect::new(inner.x + 2, inner.y + 9, inner.width.saturating_sub(4), 1),
    );
}

fn render_apikey_page(f: &mut ratatui::Frame, app: &App, inner: Rect) {
    let key = &app.settings_state.api_key;
    let n = key.chars().count();

    // Header (regular weight)
    let header_line = Line::from(vec![
        Span::styled("Edit API Key", Style::default().fg(MODAL_TITLE)),
    ]);
    let sub_line = Span::styled(
        "Authentication secret key stored locally in ~/.config/nsh/config.json.",
        Style::default().fg(MODAL_MUTED),
    );
    f.render_widget(Paragraph::new(header_line), Rect::new(inner.x, inner.y, inner.width, 1));
    f.render_widget(Paragraph::new(sub_line), Rect::new(inner.x, inner.y + 1, inner.width, 1));

    let div_str: String = "─".repeat(inner.width as usize);
    f.render_widget(Paragraph::new(div_str).style(Style::default().fg(MODAL_DIVIDER)), Rect::new(inner.x, inner.y + 2, inner.width, 1));

    // Input Card Block (white borders, regular weight title)
    let input_box_area = Rect::new(inner.x + 2, inner.y + 4, inner.width.saturating_sub(4), 3);
    let title_text = format!(" Secret Key ({} characters) ", n);
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MODAL_BORDER))
        .title(title_text)
        .title_style(Style::default().fg(Color::White));
    f.render_widget(input_block, input_box_area);

    let display_str = if n == 0 {
        "(empty - paste your API key with Ctrl+Shift+V or Ctrl+V)".to_string()
    } else {
        let bullets: String = "*".repeat(n.min(50));
        format!("{bullets} ({n} chars)")
    };

    let text_style = if n == 0 {
        Style::default().fg(MODAL_MUTED)
    } else {
        Style::default().fg(Color::White)
    };

    f.render_widget(
        Paragraph::new(format!(" {display_str}")).style(text_style),
        Rect::new(input_box_area.x + 1, input_box_area.y + 1, input_box_area.width.saturating_sub(2), 1),
    );

    let cursor_x = input_box_area.x + 2 + display_str.chars().count() as u16;
    let max_cursor_x = input_box_area.x + input_box_area.width.saturating_sub(2);
    f.set_cursor_position(Position {
        x: cursor_x.min(max_cursor_x),
        y: input_box_area.y + 1,
    });

    let env_note = "Environment fallback: $OPENAI_API_KEY, $GEMINI_API_KEY, $ANTHROPIC_API_KEY, $NSH_API_KEY";
    f.render_widget(
        Paragraph::new(env_note).style(Style::default().fg(Color::White)),
        Rect::new(inner.x + 2, inner.y + 8, inner.width.saturating_sub(4), 1),
    );
    let note2 = "Tips: Ctrl+U clears key | Paste with Ctrl+V | Enter confirms | Esc cancels";
    f.render_widget(
        Paragraph::new(note2).style(Style::default().fg(MODAL_MUTED)),
        Rect::new(inner.x + 2, inner.y + 9, inner.width.saturating_sub(4), 1),
    );
}

fn render_enable_page(f: &mut ratatui::Frame, app: &App, inner: Rect) {
    let cursor = app.settings_cursor;
    let enabled = app.settings_state.enabled;

    // Header (regular weight)
    let header_line = Line::from(vec![
        Span::styled("Enable AI Assistance", Style::default().fg(MODAL_TITLE)),
    ]);
    let sub_line = Span::styled(
        "Turn on or off ask, do, plan, and build verbs in nsh.",
        Style::default().fg(MODAL_MUTED),
    );
    f.render_widget(Paragraph::new(header_line), Rect::new(inner.x, inner.y, inner.width, 1));
    f.render_widget(Paragraph::new(sub_line), Rect::new(inner.x, inner.y + 1, inner.width, 1));

    let div_str: String = "─".repeat(inner.width as usize);
    f.render_widget(Paragraph::new(div_str).style(Style::default().fg(MODAL_DIVIDER)), Rect::new(inner.x, inner.y + 2, inner.width, 1));

    let options = [
        ("Yes", "AI assistance enabled (interactive agent)", true),
        ("No", "AI assistance disabled (pure shell mode)", false),
    ];

    let start_y = inner.y + 4;
    for (i, (_tag, desc, val)) in options.iter().enumerate() {
        let y = start_y + i as u16 * 2;
        if y + 1 >= inner.y + inner.height {
            break;
        }

        let is_selected = cursor == i;
        let is_current = *val == enabled;

        let radio_str = if is_current { "(*) " } else { "( ) " };

        if is_selected {
            f.render_widget(
                Block::default().style(Style::default().bg(MODAL_SELECTED_BG)),
                Rect::new(inner.x, y, inner.width, 2),
            );
        }

        let row_style = if is_selected {
            Style::default().fg(Color::White).bg(MODAL_SELECTED_BG)
        } else {
            Style::default().fg(Color::White)
        };

        let badge_text = if *val { "[ ON  ] " } else { "[ OFF ] " };
        let active_tag = if is_current { " (current)" } else { "" };
        let line = Line::from(vec![
            Span::raw("  "),
            Span::styled(radio_str, row_style),
            Span::styled(badge_text, row_style),
            Span::styled(*desc, row_style),
            Span::styled(active_tag, if is_selected { row_style } else { Style::default().fg(MODAL_MUTED) }),
        ]);
        f.render_widget(Paragraph::new(line), Rect::new(inner.x, y, inner.width, 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_compute_settings_modal_area_clamping() {
        // Large screen (120x40)
        let large_screen = Rect::new(0, 0, 120, 40);
        let modal = compute_settings_modal_area(large_screen);
        assert!(modal.width <= 92);
        assert!(modal.width >= 52);
        assert!(modal.height <= 28);
        assert!(modal.height >= 18);
        // Centered
        assert_eq!(modal.x, (120 - modal.width) / 2);
        assert_eq!(modal.y, (40 - modal.height) / 2);

        // Small screen (40x12)
        let small_screen = Rect::new(0, 0, 40, 12);
        let modal_small = compute_settings_modal_area(small_screen);
        assert_eq!(modal_small.width, 40);
        assert_eq!(modal_small.height, 12);
        assert_eq!(modal_small.x, 0);
        assert_eq!(modal_small.y, 0);
    }

    #[test]
    fn test_render_settings_modal_drawing() {
        let backend = TestBackend::new(90, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.show_settings = true;

        terminal
            .draw(|f| {
                render_settings(f, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let buffer_str = (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(buffer_str.contains("Settings"));
        assert!(buffer_str.contains("Provider"));
        assert!(buffer_str.contains("Model"));
        assert!(buffer_str.contains("Base URL"));
        assert!(buffer_str.contains("API Key"));
        assert!(buffer_str.contains("Enable AI"));
        assert!(buffer_str.contains("Save & Apply"));
    }

    #[test]
    fn test_render_settings_subpages() {
        let backend = TestBackend::new(90, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.show_settings = true;

        // Provider page
        app.settings_push(SettingsPage::Provider);
        terminal
            .draw(|f| {
                render_settings(f, &app);
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let text = format!("{:?}", buf);
        assert!(text.contains("Select AI Provider"));

        // BaseUrl page
        app.settings_push(SettingsPage::BaseUrl);
        terminal
            .draw(|f| {
                render_settings(f, &app);
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let text = format!("{:?}", buf);
        assert!(text.contains("Edit Base URL"));

        // ApiKey page
        app.settings_push(SettingsPage::ApiKey);
        terminal
            .draw(|f| {
                render_settings(f, &app);
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let text = format!("{:?}", buf);
        assert!(text.contains("Edit API Key"));

        // Enable page
        app.settings_push(SettingsPage::Enable);
        terminal
            .draw(|f| {
                render_settings(f, &app);
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let text = format!("{:?}", buf);
        assert!(text.contains("Enable AI Assistance"));
    }

    #[test]
    fn test_settings_filter_filtering() {
        let backend = TestBackend::new(90, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.show_settings = true;
        app.settings_filter = "provider".to_string();

        terminal
            .draw(|f| {
                render_settings(f, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let buffer_str = (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(buffer_str.contains("Provider"));

        app.settings_reset_filter();
        assert!(app.settings_filter.is_empty());
        assert!(!app.settings_filter_active);
    }
}
