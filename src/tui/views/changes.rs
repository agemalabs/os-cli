//! Changes view — pending write-backs with approve/reject.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::tui::theme;

/// Changes view state.
#[derive(Debug, Clone, Default)]
pub struct ChangesState {
    pub changes: Vec<PendingChange>,
    pub selected: usize,
    pub loaded: bool,
}

/// A pending change for display.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PendingChange {
    pub id: String,
    pub file_title: String,
    pub project: String,
    pub reason: String,
    pub diff: String,
}

/// Fetch pending changes from the API.
pub async fn fetch(client: &crate::api_client::ApiClient) -> anyhow::Result<Vec<PendingChange>> {
    let resp: serde_json::Value = client.get("/changes").await?;

    let changes = resp["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|c| PendingChange {
                    id: c["id"].as_str().unwrap_or("").to_string(),
                    file_title: c["file_id"].as_str().unwrap_or("file").to_string(),
                    project: String::new(),
                    reason: c["reason"].as_str().unwrap_or("").to_string(),
                    diff: c["diff"].as_str().unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(changes)
}

/// Approve a change.
pub async fn approve(client: &crate::api_client::ApiClient, id: &str) -> anyhow::Result<()> {
    let _: serde_json::Value = client
        .post(&format!("/changes/{}/approve", id), &serde_json::json!({}))
        .await?;
    Ok(())
}

/// Reject a change.
pub async fn reject(client: &crate::api_client::ApiClient, id: &str) -> anyhow::Result<()> {
    let _: serde_json::Value = client
        .post(&format!("/changes/{}/reject", id), &serde_json::json!({}))
        .await?;
    Ok(())
}

/// Render the changes view.
pub fn render(frame: &mut Frame, area: Rect, state: &ChangesState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    // Header
    let count = state.changes.len();
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" PENDING CHANGES", theme::title_style()),
        Span::styled(
            format!(
                "  {} write-back{} awaiting review",
                count,
                if count == 1 { "" } else { "s" }
            ),
            theme::muted_style(),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(header, chunks[0]);

    // Content
    render_changes(frame, chunks[1], state);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" a", theme::key_style()),
        Span::styled("·approve  ", theme::muted_style()),
        Span::styled("r", theme::key_style()),
        Span::styled("·reject  ", theme::muted_style()),
        Span::styled("j/k", theme::key_style()),
        Span::styled("·navigate  ", theme::muted_style()),
        Span::styled("b", theme::key_style()),
        Span::styled("·back", theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(footer, chunks[2]);
}

fn render_changes(frame: &mut Frame, area: Rect, state: &ChangesState) {
    let mut lines = Vec::new();

    if !state.loaded {
        lines.push(Line::from(Span::styled(
            "  Loading...",
            theme::muted_style(),
        )));
    } else if state.changes.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  No pending changes.",
            theme::muted_style(),
        )));
    } else {
        for (i, change) in state.changes.iter().enumerate() {
            let is_selected = i == state.selected;
            let marker = if is_selected {
                theme::MARKER_ACTIVE
            } else {
                theme::MARKER_OS
            };
            let style = if is_selected {
                theme::active_style()
            } else {
                theme::label_style()
            };

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), theme::warning_style()),
                Span::styled(&change.file_title, style),
            ]));

            if !change.reason.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("    \"{}\"", truncate(&change.reason, 70)),
                    theme::muted_style(),
                )));
            }

            // Show diff lines if selected
            if is_selected && !change.diff.is_empty() {
                lines.push(Line::from(""));
                for diff_line in change.diff.lines().take(10) {
                    let diff_style = if diff_line.starts_with('+') {
                        theme::success_style()
                    } else if diff_line.starts_with('-') {
                        theme::danger_style()
                    } else {
                        theme::muted_style()
                    };
                    lines.push(Line::from(Span::styled(
                        format!("    {}", diff_line),
                        diff_style,
                    )));
                }
            }
        }
    }

    let widget = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max - 3])
    } else {
        s.to_string()
    }
}
