//! Search view — interactive semantic search with results.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::tui::theme;

/// Search view state.
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected: usize,
    pub searching: bool,
    pub searched: bool,
}

/// A single search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub project_slug: String,
    pub file_slug: String,
    pub chunk_text: String,
    pub similarity: f64,
}

/// Execute a search against the API.
pub async fn execute(
    client: &crate::api_client::ApiClient,
    query: &str,
) -> anyhow::Result<Vec<SearchResult>> {
    let body = serde_json::json!({
        "query": query,
        "limit": 10
    });

    let resp: serde_json::Value = client.post("/search", &body).await?;

    let results = resp["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|r| SearchResult {
                    project_slug: r["project_slug"].as_str().unwrap_or("?").to_string(),
                    file_slug: r["file_slug"].as_str().unwrap_or("?").to_string(),
                    chunk_text: r["chunk_text"].as_str().unwrap_or("").to_string(),
                    similarity: r["similarity"].as_f64().unwrap_or(0.0),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(results)
}

/// Render the search view.
pub fn render(frame: &mut Frame, area: Rect, state: &SearchState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(3), // search input
            Constraint::Min(6),    // results
            Constraint::Length(3), // footer
        ])
        .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        " SEARCH",
        theme::title_style(),
    )]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(header, chunks[0]);

    // Search input
    let cursor = if state.searching { "" } else { "█" };
    let input = Paragraph::new(Line::from(vec![
        Span::styled("  > ", theme::active_style()),
        Span::styled(&state.query, theme::label_style()),
        Span::styled(cursor, theme::active_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(input, chunks[1]);

    // Results
    render_results(frame, chunks[2], state);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Enter", theme::key_style()),
        Span::styled("·search  ", theme::muted_style()),
        Span::styled("j/k", theme::key_style()),
        Span::styled("·navigate  ", theme::muted_style()),
        Span::styled("Esc", theme::key_style()),
        Span::styled("·clear  ", theme::muted_style()),
        Span::styled("b", theme::key_style()),
        Span::styled("·back", theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(footer, chunks[3]);
}

fn render_results(frame: &mut Frame, area: Rect, state: &SearchState) {
    let mut lines = Vec::new();

    if state.searching {
        lines.push(Line::from(Span::styled(
            "  Searching...",
            theme::muted_style(),
        )));
    } else if !state.searched {
        lines.push(Line::from(Span::styled(
            "  Type a query and press Enter",
            theme::muted_style(),
        )));
    } else if state.results.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No results found.",
            theme::muted_style(),
        )));
    } else {
        for (i, result) in state.results.iter().enumerate() {
            let marker = if i == state.selected {
                theme::MARKER_ACTIVE
            } else {
                theme::MARKER_OPEN
            };

            let style = if i == state.selected {
                theme::active_style()
            } else {
                theme::label_style()
            };

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), style),
                Span::styled(
                    format!("{} · {}", result.project_slug, result.file_slug),
                    style,
                ),
                Span::styled(format!("  {:.2}", result.similarity), theme::muted_style()),
            ]));

            // Show preview (first 120 chars)
            let preview = result.chunk_text.replace('\n', " ");
            let preview = if preview.len() > 120 {
                format!("{}...", &preview[..120])
            } else {
                preview
            };
            lines.push(Line::from(Span::styled(
                format!("    \"{}\"", preview),
                theme::muted_style(),
            )));
        }
    }

    let widget = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}
