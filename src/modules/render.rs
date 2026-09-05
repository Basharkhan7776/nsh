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
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
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
            if app.show_sudo_prompt {
                render_sudo_password_modal(f, app);
            } else if app.show_history_modal {
                render_history_modal(f, app);
            }
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

pub fn extract_selected_text(
    app: &App,
    width: usize,
    list_area_y: u16,
    visible_height: usize,
) -> Option<String> {
    let sel = app.selection?;
    if sel.start == sel.end {
        return None;
    }

    let ((r1, c1), (r2, c2)) = if sel.start.1 < sel.end.1 || (sel.start.1 == sel.end.1 && sel.start.0 <= sel.end.0) {
        ((sel.start.1, sel.start.0), (sel.end.1, sel.end.0))
    } else {
        ((sel.end.1, sel.end.0), (sel.start.1, sel.start.0))
    };

    let display_rows = build_display_rows(app, width);
    let mut selected_lines = Vec::new();

    for y in r1..=r2 {
        if y < list_area_y {
            continue;
        }
        let rel_y = (y - list_area_y) as usize;
        if rel_y >= visible_height {
            break;
        }
        let row_idx = app.scroll_offset + rel_y;
        if row_idx >= display_rows.len() {
            break;
        }

        let text = &display_rows[row_idx].text;
        let char_count = text.chars().count();
        if char_count == 0 {
            selected_lines.push(String::new());
            continue;
        }

        let from_col = if y == r1 { (c1 as usize).min(char_count) } else { 0 };
        let to_col = if y == r2 { ((c2 as usize) + 1).min(char_count) } else { char_count };

        if from_col < to_col {
            let slice: String = text.chars().skip(from_col).take(to_col - from_col).collect();
            selected_lines.push(slice);
        } else {
            selected_lines.push(String::new());
        }
    }

    if selected_lines.is_empty() || (selected_lines.len() == 1 && selected_lines[0].is_empty()) {
        None
    } else {
        Some(selected_lines.join("\n"))
    }
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

    let (list_area, scrollbar_area) = if chunks[0].width > 2 {
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(chunks[0]);
        (h_chunks[0], Some(h_chunks[1]))
    } else {
        (chunks[0], None)
    };

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

    let sel_coords = app.selection.map(|sel| {
        if sel.start.1 < sel.end.1 || (sel.start.1 == sel.end.1 && sel.start.0 <= sel.end.0) {
            ((sel.start.1, sel.start.0), (sel.end.1, sel.end.0))
        } else {
            ((sel.end.1, sel.end.0), (sel.start.1, sel.start.0))
        }
    });
    let sel_style = Style::default().bg(Color::Rgb(60, 60, 60)).fg(Color::White);

    let mut items: Vec<ListItem> = Vec::new();
    for (y_idx, row) in display_rows.iter().take(end_line).skip(start_line).enumerate() {
        let screen_y = list_area.y + (y_idx as u16);
        let base_style = match row.kind {
            EntryType::Command => cmd_style,
            EntryType::Output => output_fg,
            EntryType::System => system_style,
        };

        let is_selected = sel_coords.map_or(false, |((r1, _), (r2, _))| screen_y >= r1 && screen_y <= r2);

        let line = if is_selected {
            if let Some(((r1, c1), (r2, c2))) = sel_coords {
                let char_count = row.text.chars().count();
                let from_col = if screen_y == r1 { (c1 as usize).min(char_count) } else { 0 };
                let to_col = if screen_y == r2 { ((c2 as usize) + 1).min(char_count) } else { char_count };

                if from_col < to_col && from_col < char_count {
                    let before: String = row.text.chars().take(from_col).collect();
                    let selected: String = row.text.chars().skip(from_col).take(to_col - from_col).collect();
                    let after: String = row.text.chars().skip(to_col).collect();

                    Line::from(vec![
                        Span::styled(before, base_style),
                        Span::styled(selected, sel_style),
                        Span::styled(after, base_style),
                    ])
                } else {
                    Line::from(Span::styled(row.text.as_str(), base_style))
                }
            } else {
                Line::from(Span::styled(row.text.as_str(), base_style))
            }
        } else {
            match row.kind {
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
            }
        };
        items.push(ListItem::new(line));
    }

    // Blank list_area with output_bg so previous lines are never left behind on clear
    f.render_widget(Block::default().style(output_bg), list_area);
    let list = List::new(items)
        .block(Block::default().style(output_bg))
        .style(output_bg);
    f.render_widget(list, list_area);

    // Render rightmost scrollbar gutter and track
    if let Some(sb_area) = scrollbar_area {
        f.render_widget(Block::default().style(output_bg), sb_area);
        if content_height > visible_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(None)
                .thumb_symbol("▕")
                .thumb_style(Style::default().fg(Color::Rgb(90, 90, 90)).bg(OUTPUT_BG));

            let mut scrollbar_state = ScrollbarState::new(content_height.saturating_sub(visible_height))
                .position(app.scroll_offset);
            f.render_stateful_widget(scrollbar, sb_area, &mut scrollbar_state);
        }
    }

    // Animated AI loading bar (ask / do / plan / build).
    if let (Some(area), Some(loading)) = (status_area, app.ai_loading.as_ref()) {
        let text = format!(" >  {}", loading.status_line());
        f.render_widget(Paragraph::new(text).style(loading_style), area);
    }

    // Real terminal cursor (no fake "|" glyph). Top padding is 1 row.
    // While AI is loading or in Output focus, dim/inform the input area.
    let pad_top: u16 = 1;
    let pad_left: u16 = 0;
    let cursor_byte = app.cursor_position.min(app.current_input.len());
    let before = &app.current_input[..cursor_byte];
    let is_plan_review = app.active_plan_session.is_some();
    let is_placeholder = app.ai_loading.is_some()
        || app.focus == super::state::Focus::Output
        || (is_plan_review && app.current_input.is_empty());

    let (prompt, body) = if app.ai_loading.is_some() {
        (" … ", "(AI is working — wait for response)")
    } else if app.focus == super::state::Focus::Output {
        ("", "Press i or enter to type")
    } else if is_plan_review {
        if app.current_input.is_empty() {
            ("> [plan] ", "Type 'approve', 'deny', or suggestion to refine...")
        } else {
            ("> [plan] ", app.current_input.as_str())
        }
    } else {
        (PROMPT_TEXT, app.current_input.as_str())
    };

    let prompt_cols = prompt.chars().count() as u16;
    // Available horizontal width for the text portion (single line, horizontal extend & scroll)
    let text_avail = (input_area.width.saturating_sub(pad_left + prompt_cols + 1) as usize).max(1);

    let body_chars: Vec<char> = body.chars().collect();
    let total_chars = body_chars.len();

    let cursor_char_idx = if app.focus == super::state::Focus::Input && app.ai_loading.is_none() && !is_placeholder {
        before.chars().count()
    } else {
        0
    };

    // Calculate horizontal scroll in X so cursor and long input stay visible without wrapping
    if app.focus == super::state::Focus::Input && app.ai_loading.is_none() && !is_placeholder {
        if cursor_char_idx < app.input_scroll_x {
            app.input_scroll_x = cursor_char_idx;
        } else if cursor_char_idx >= app.input_scroll_x + text_avail {
            app.input_scroll_x = cursor_char_idx.saturating_sub(text_avail) + 1;
        }
        if total_chars <= text_avail {
            app.input_scroll_x = 0;
        } else {
            let max_scroll = total_chars.saturating_sub(text_avail);
            if app.input_scroll_x > max_scroll && cursor_char_idx <= total_chars {
                app.input_scroll_x = max_scroll;
            }
        }
    } else {
        app.input_scroll_x = 0;
    }

    let start_idx = app.input_scroll_x.min(total_chars);
    let end_idx = (start_idx + text_avail).min(total_chars);
    let visible_body: String = body_chars[start_idx..end_idx].iter().collect();

    let body_style = if is_placeholder {
        Style::default().fg(Color::Rgb(120, 120, 120)).bg(INPUT_BG)
    } else {
        Style::default().fg(OUTPUT_FG).bg(INPUT_BG)
    };

    let input_line = Line::from(vec![
        Span::styled(prompt, input_style),
        Span::styled(visible_body, body_style),
    ]);
    let input_block = Block::default()
        .style(input_style)
        .padding(ratatui::widgets::Padding::new(pad_left, 0, pad_top, 0));
    let input_widget = Paragraph::new(input_line)
        .style(input_style)
        .block(input_block);
    f.render_widget(input_widget, input_area);

    // Render horizontal scrollbar in X when input exceeds available width
    if total_chars > text_avail
        && app.focus == super::state::Focus::Input
        && app.ai_loading.is_none()
        && input_area.height >= 3
    {
        let sb_y = input_area.y.saturating_add(2);
        let sb_x = input_area.x.saturating_add(pad_left).saturating_add(prompt_cols);
        let sb_w = input_area.width.saturating_sub(pad_left + prompt_cols);
        if sb_w > 2 {
            let sb_area = Rect::new(sb_x, sb_y, sb_w, 1);
            let x_scrollbar = Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(Some("─"))
                .track_style(Style::default().fg(Color::Rgb(50, 50, 50)).bg(INPUT_BG))
                .thumb_symbol("━")
                .thumb_style(Style::default().fg(Color::Rgb(160, 160, 160)).bg(INPUT_BG));

            let max_scroll = total_chars.saturating_sub(text_avail);
            let mut x_scrollbar_state = ScrollbarState::new(max_scroll)
                .position(app.input_scroll_x);
            f.render_stateful_widget(x_scrollbar, sb_area, &mut x_scrollbar_state);
        }
    }

    // Hide cursor while AI is loading, output is focused, or a modal (sudo / history) is active.
    if app.ai_loading.is_none()
        && app.focus == super::state::Focus::Input
        && !app.show_sudo_prompt
        && !app.show_history_modal
    {
        let visible_cursor_offset = cursor_char_idx.saturating_sub(start_idx) as u16;
        let cursor_x = input_area
            .x
            .saturating_add(pad_left)
            .saturating_add(prompt_cols)
            .saturating_add(visible_cursor_offset);
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

    if app.ai_loading.is_none()
        && app.focus == super::state::Focus::Input
        && app.show_suggestions
        && !app.show_history_modal
        && !app.current_suggestions.is_empty()
    {
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

pub fn compute_sudo_modal_area(screen: Rect) -> Rect {
    let target_w: u16 = 54;
    let target_h: u16 = 9;

    let width = target_w.min(screen.width.saturating_sub(4)).max(32);
    let height = target_h.min(screen.height.saturating_sub(2)).max(7);

    let x = (screen.width.saturating_sub(width)) / 2;
    let y = (screen.height.saturating_sub(height)) / 2;

    Rect::new(x, y, width, height)
}

pub fn render_sudo_password_modal(f: &mut ratatui::Frame, app: &App) {
    let modal_area = compute_sudo_modal_area(f.area());

    // Clear cells underneath so background text doesn't bleed through
    f.render_widget(Clear, modal_area);

    let output_bg = Style::default().bg(OUTPUT_BG);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .style(output_bg);
    f.render_widget(block, modal_area);

    // Title on top border (no emojis, white, clean)
    let title_line = Line::from(Span::styled(
        " Authentication Required ",
        Style::default().fg(Color::White),
    ));
    f.render_widget(
        Paragraph::new(title_line),
        Rect::new(modal_area.x + 2, modal_area.y, modal_area.width.saturating_sub(4), 1),
    );

    let inner = Rect::new(
        modal_area.x + 2,
        modal_area.y + 1,
        modal_area.width.saturating_sub(4),
        modal_area.height.saturating_sub(2),
    );

    let cmd_raw = app.pending_sudo_command.as_deref().unwrap_or("sudo");
    let max_cmd_len = inner.width.saturating_sub(12) as usize;
    let cmd_display = if cmd_raw.len() > max_cmd_len && max_cmd_len > 3 {
        format!("{}...", &cmd_raw[..max_cmd_len - 3])
    } else {
        cmd_raw.to_string()
    };

    let user = std::env::var("USER").unwrap_or_else(|_| "root".to_string());
    let bullets = "•".repeat(app.sudo_password.chars().count());

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Command:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(cmd_display, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("User:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(user, Style::default().fg(Color::White)),
        ]),
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled("Password: ", Style::default().fg(Color::White)),
            Span::styled(bullets, Style::default().fg(Color::White)),
        ]),
    ];

    if let Some(err) = &app.sudo_error {
        lines.push(Line::from(Span::styled(
            format!(" {}", err),
            Style::default().fg(Color::Rgb(220, 80, 80)),
        )));
    } else {
        lines.push(Line::from(Span::raw("")));
    }

    lines.push(Line::from(vec![
        Span::styled("[Enter] ", Style::default().fg(Color::White)),
        Span::styled("Submit   ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Esc] ", Style::default().fg(Color::White)),
        Span::styled("Cancel", Style::default().fg(Color::DarkGray)),
    ]));

    f.render_widget(Paragraph::new(lines).style(output_bg), inner);

    // Position the real terminal cursor at the end of the password bullets
    let pass_len = app.sudo_password.chars().count() as u16;
    let cursor_x = (inner.x + 10 + pass_len).min(inner.x + inner.width.saturating_sub(1));
    let cursor_y = inner.y + 3;
    f.set_cursor_position(Position {
        x: cursor_x,
        y: cursor_y,
    });
}

pub fn compute_history_modal_area(screen: Rect) -> Rect {
    let target_w: u16 = (screen.width as f32 * 0.72) as u16;
    let target_h: u16 = (screen.height as f32 * 0.70) as u16;

    let width = target_w.clamp(48, 86).min(screen.width.saturating_sub(4));
    let height = target_h.clamp(12, 22).min(screen.height.saturating_sub(2));

    let x = (screen.width.saturating_sub(width)) / 2;
    let y = (screen.height.saturating_sub(height)) / 2;

    Rect::new(x, y, width, height)
}

pub fn render_history_modal(f: &mut ratatui::Frame, app: &App) {
    let modal_area = compute_history_modal_area(f.area());

    // Clear cells underneath so background text doesn't bleed through
    f.render_widget(Clear, modal_area);

    let output_bg = Style::default().bg(OUTPUT_BG);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .style(output_bg);
    f.render_widget(block, modal_area);

    let cmds = app.filtered_history_commands();
    let total_count = app
        .command_history
        .iter()
        .filter(|c| {
            let t = c.trim();
            !t.eq_ignore_ascii_case("history") && !t.eq_ignore_ascii_case("/history")
        })
        .count();

    // Title on top border (monochrome, zero emojis)
    let title_str = if app.history_modal_filter.is_empty() {
        format!(" Command History ({} commands) ", cmds.len())
    } else {
        format!(" Command History ({}/{} matches) ", cmds.len(), total_count)
    };
    let title_line = Line::from(Span::styled(
        title_str,
        Style::default().fg(Color::White),
    ));
    f.render_widget(
        Paragraph::new(title_line),
        Rect::new(modal_area.x + 2, modal_area.y, modal_area.width.saturating_sub(14), 1),
    );

    // Top-right close button
    let close_hint = "[Esc Close]";
    f.render_widget(
        Paragraph::new(close_hint).style(Style::default().fg(Color::DarkGray)),
        Rect::new(modal_area.x + modal_area.width.saturating_sub(12), modal_area.y, 11, 1),
    );

    let inner = Rect::new(
        modal_area.x + 2,
        modal_area.y + 1,
        modal_area.width.saturating_sub(4),
        modal_area.height.saturating_sub(2),
    );

    // Search / Filter line
    let filter_line = if app.history_modal_filter.is_empty() {
        Line::from(vec![
            Span::styled("Filter: ", Style::default().fg(Color::White)),
            Span::styled("Type to search history...", Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(vec![
            Span::styled("Filter: ", Style::default().fg(Color::White)),
            Span::styled(&app.history_modal_filter, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("_", Style::default().fg(Color::DarkGray)),
        ])
    };
    f.render_widget(Paragraph::new(filter_line).style(output_bg), Rect::new(inner.x, inner.y, inner.width, 1));

    // Separator line
    let sep = "─".repeat(inner.width as usize);
    f.render_widget(
        Paragraph::new(Span::styled(sep, Style::default().fg(Color::DarkGray))).style(output_bg),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    // History list
    let list_y = inner.y + 2;
    let list_h = inner.height.saturating_sub(3) as usize; // reserve 1 row for bottom footer

    let scroll = if app.history_modal_selected < app.history_modal_scroll {
        app.history_modal_selected
    } else if app.history_modal_selected >= app.history_modal_scroll + list_h {
        app.history_modal_selected + 1 - list_h
    } else {
        app.history_modal_scroll
    };

    let mut list_lines = Vec::new();
    if cmds.is_empty() {
        if app.command_history.is_empty() {
            list_lines.push(Line::from(Span::styled(
                "  (No command history recorded yet)",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            list_lines.push(Line::from(Span::styled(
                format!("  No commands matching \"{}\"", app.history_modal_filter),
                Style::default().fg(Color::DarkGray),
            )));
        }
    } else {
        let visible_items = cmds.iter().enumerate().skip(scroll).take(list_h);
        for (i, cmd) in visible_items {
            let is_selected = i == app.history_modal_selected;
            let num = i + 1;
            let max_cmd_len = (inner.width as usize).saturating_sub(10);
            let display_cmd = if cmd.chars().count() > max_cmd_len && max_cmd_len > 3 {
                let s: String = cmd.chars().take(max_cmd_len - 3).collect();
                format!("{}...", s)
            } else {
                cmd.clone()
            };

            if is_selected {
                list_lines.push(
                    Line::from(vec![
                        Span::styled(format!("> {:>3}  ", num), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled(display_cmd, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ])
                    .style(Style::default().bg(Color::Rgb(40, 40, 40))),
                );
            } else {
                list_lines.push(Line::from(vec![
                    Span::styled(format!("  {:>3}  ", num), Style::default().fg(Color::DarkGray)),
                    Span::styled(display_cmd, Style::default().fg(Color::White)),
                ]));
            }
        }
    }

    f.render_widget(Paragraph::new(list_lines).style(output_bg), Rect::new(inner.x, list_y, inner.width, list_h as u16));

    // Footer at bottom of inner area
    let footer_y = inner.y + inner.height.saturating_sub(1);
    let footer_line = Line::from(vec![
        Span::styled("[Up/Down] ", Style::default().fg(Color::White)),
        Span::styled("Select   ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Enter] ", Style::default().fg(Color::White)),
        Span::styled("Enter in Input   ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Esc] ", Style::default().fg(Color::White)),
        Span::styled("Close", Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(
        Paragraph::new(footer_line).centered().style(output_bg),
        Rect::new(inner.x, footer_y, inner.width, 1),
    );

    // Position the real terminal cursor at the filter line
    let cursor_x = (inner.x + 8 + app.history_modal_filter.chars().count() as u16)
        .min(inner.x + inner.width.saturating_sub(1));
    f.set_cursor_position(Position {
        x: cursor_x,
        y: inner.y,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Entry, Focus, Selection};
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

    #[test]
    fn test_render_shell_right_scrollbar() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        // Add enough entries so content_height > visible_height
        for i in 0..40 {
            app.add_entry(Entry {
                entry_type: EntryType::Output,
                content: vec![format!("Line output {}", i)],
                cwd: String::new(),
            });
        }

        terminal
            .draw(|f| {
                render_shell(f, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        // Check that rightmost column (col 79) contains scrollbar track or thumb symbols
        let right_col = buffer.area.width - 1;
        let col_symbols: Vec<&str> = (0..17)
            .map(|y| buffer[(right_col, y)].symbol())
            .collect();
        assert!(
            col_symbols.iter().any(|&s| s == "▕"),
            "Rightmost column should contain minimal scrollbar thumb '▕', got: {:?}",
            col_symbols
        );
    }

    #[test]
    fn test_extract_selected_text_single_and_multi_line() {
        let mut app = App::new();
        app.add_entry(Entry {
            entry_type: EntryType::Output,
            content: vec![
                "first line of text".to_string(),
                "second line of text".to_string(),
                "third line of text".to_string(),
            ],
            cwd: String::new(),
        });

        // Single line selection: "line" from "first line of text" (columns 6..9 inclusive)
        app.selection = Some(Selection {
            start: (6, 0),
            end: (9, 0),
        });
        let extracted = extract_selected_text(&app, 80, 0, 10);
        assert_eq!(extracted, Some("line".to_string()));

        // Multi-line selection: from column 6 on row 0 to column 5 on row 1
        app.selection = Some(Selection {
            start: (6, 0),
            end: (5, 1),
        });
        let multi = extract_selected_text(&app, 80, 0, 10);
        assert_eq!(multi, Some("line of text\nsecond".to_string()));

        // Single point click (no drag) returns None
        app.selection = Some(Selection {
            start: (3, 0),
            end: (3, 0),
        });
        assert_eq!(extract_selected_text(&app, 80, 0, 10), None);
    }

    #[test]
    fn test_render_output_focus_and_selection() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.focus = Focus::Output;
        app.add_entry(Entry {
            entry_type: EntryType::Output,
            content: vec!["alpha beta gamma".to_string()],
            cwd: String::new(),
        });
        app.selection = Some(Selection {
            start: (0, 0),
            end: (4, 0),
        });

        terminal
            .draw(|f| {
                render_shell(f, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let text = format!("{:?}", buffer);
        assert!(text.contains("Press i or enter to type"));
    }

    #[test]
    fn test_compute_sudo_modal_area() {
        let screen = Rect::new(0, 0, 100, 30);
        let area = compute_sudo_modal_area(screen);
        assert_eq!(area.width, 54);
        assert_eq!(area.height, 9);
        assert_eq!(area.x, (100 - 54) / 2);
        assert_eq!(area.y, (30 - 9) / 2);

        // Small screen clamping
        let small = Rect::new(0, 0, 40, 10);
        let small_area = compute_sudo_modal_area(small);
        assert!(small_area.width <= 40);
        assert!(small_area.height <= 10);
    }

    #[test]
    fn test_render_sudo_password_modal() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.show_sudo_prompt = true;
        app.pending_sudo_command = Some("sudo apt update".to_string());
        app.sudo_password = "secretpass".to_string();

        terminal
            .draw(|f| {
                render_sudo_password_modal(f, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let text = format!("{:?}", buffer);
        assert!(text.contains("Authentication Required"));
        assert!(text.contains("Command:"));
        assert!(text.contains("sudo apt update"));
        assert!(text.contains("Password:"));
        // Check that bullets are rendered
        assert!(text.contains("••••••••••"));
        assert!(text.contains("[Enter]"));
        assert!(text.contains("[Esc]"));
    }

    #[test]
    fn test_render_input_horizontal_scroll_and_scrollbar() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        // Set a long input exceeding the 40-col screen width
        let long_input = "build make a landing page from @index.html with 2 sections";
        app.current_input = long_input.to_string();
        app.cursor_position = long_input.len();

        terminal
            .draw(|f| {
                render_shell(f, &mut app);
            })
            .unwrap();

        // Horizontal scroll offset must have shifted so the end is visible
        assert!(app.input_scroll_x > 0, "app.input_scroll_x should be > 0 for long input");

        let buffer = terminal.backend().buffer();
        let text = format!("{:?}", buffer);
        // Should contain end of long input (e.g. "sections")
        assert!(text.contains("sections"), "Visible input should display scrolled content");
        // Should contain the horizontal scrollbar thumb symbol
        assert!(text.contains('━'), "Horizontal scrollbar in X should be rendered");
    }

    #[test]
    fn test_compute_history_modal_area_clamping() {
        let large_screen = Rect::new(0, 0, 120, 40);
        let modal = compute_history_modal_area(large_screen);
        assert!(modal.width <= 86 && modal.width >= 48);
        assert!(modal.height <= 22 && modal.height >= 12);
        assert_eq!(modal.x, (120 - modal.width) / 2);
        assert_eq!(modal.y, (40 - modal.height) / 2);

        let small_screen = Rect::new(0, 0, 40, 10);
        let modal_small = compute_history_modal_area(small_screen);
        assert!(modal_small.width <= 40);
        assert!(modal_small.height <= 10);
    }

    #[test]
    fn test_render_history_modal() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.command_history = vec![
            "git status".to_string(),
            "cargo test".to_string(),
            "npm run dev".to_string(),
        ];
        app.open_history_modal();
        assert!(app.show_history_modal);

        terminal
            .draw(|f| {
                render_history_modal(f, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let text = format!("{:?}", buffer);
        assert!(text.contains("Command History"));
        assert!(text.contains("Filter:"));
        assert!(text.contains("[Esc Close]"));
        assert!(text.contains("git status"));
        assert!(text.contains("cargo test"));
        assert!(text.contains("npm run dev"));
        assert!(text.contains("[Up/Down]"));
        assert!(text.contains("[Enter]"));
    }
}
