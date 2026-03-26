// UI rendering for terminal shell

use super::commands::shorten_cwd;
use super::config::*;
use super::state::{App, EntryType};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
    Terminal,
};

// Main render function - draws all UI components
pub fn render(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &App,
) -> std::io::Result<()> {
    terminal.draw(|f| {
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
    })?;
    Ok(())
}
