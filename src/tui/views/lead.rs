//! Lead detail view.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::tui::theme;

/// Lead detail data.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct LeadDetail {
    pub id: String,
    pub company_name: String,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub estimated_value: Option<String>,
    pub stage: String,
    pub source: Option<String>,
    pub source_detail: Option<String>,
    pub next_action: Option<String>,
    pub next_action_date: Option<String>,
    pub notes: Vec<LeadNote>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LeadNote {
    pub content: String,
    pub created_at: String,
}

/// Fetch lead detail from the API.
pub async fn fetch(client: &crate::api_client::ApiClient, id: &str) -> anyhow::Result<LeadDetail> {
    let resp: serde_json::Value = client.get(&format!("/leads/{}", id)).await?;
    let d = &resp["data"];

    let notes = d["notes"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|n| LeadNote {
                    content: n["content"].as_str().unwrap_or("").to_string(),
                    created_at: n["created_at"].as_str().unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(LeadDetail {
        id: d["id"].as_str().unwrap_or("").to_string(),
        company_name: d["company_name"].as_str().unwrap_or("?").to_string(),
        contact_name: d["contact_name"].as_str().map(|s| s.to_string()),
        contact_email: d["contact_email"].as_str().map(|s| s.to_string()),
        estimated_value: d["estimated_value"].as_f64().map(|v| format!("${:.0}", v)),
        stage: d["stage"].as_str().unwrap_or("cold").to_string(),
        source: d["source"].as_str().map(|s| s.to_string()),
        source_detail: d["source_detail"].as_str().map(|s| s.to_string()),
        next_action: d["next_action"].as_str().map(|s| s.to_string()),
        next_action_date: d["next_action_date"].as_str().map(|s| s.to_string()),
        notes,
    })
}

/// Render the lead detail view.
pub fn render(frame: &mut Frame, area: Rect, data: &LeadDetail) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // header
            Constraint::Length(4), // next action
            Constraint::Min(6),    // notes
            Constraint::Length(3), // footer
        ])
        .split(area);

    // Header
    let contact = data.contact_name.as_deref().unwrap_or("—");
    let email = data.contact_email.as_deref().unwrap_or("");
    let value = data.estimated_value.as_deref().unwrap_or("—");
    let source = data.source.as_deref().unwrap_or("—");

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" LEAD ", theme::title_style()),
            Span::styled(format!("· {}", data.company_name), theme::muted_style()),
        ]),
        Line::from(vec![Span::styled(
            format!("  Contact: {} · {}", contact, email),
            theme::label_style(),
        )]),
        Line::from(vec![Span::styled(
            format!(
                "  Value: {}   Stage: {}   Source: {}",
                value, data.stage, source
            ),
            theme::muted_style(),
        )]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(header, chunks[0]);

    // Next action
    let action = data.next_action.as_deref().unwrap_or("None");
    let action_date = data.next_action_date.as_deref().unwrap_or("");
    let is_overdue = !action_date.is_empty(); // simplified — real check would compare dates

    let action_style = if is_overdue {
        theme::warning_style()
    } else {
        theme::label_style()
    };

    let next = Paragraph::new(vec![
        Line::from(Span::styled("  NEXT ACTION", theme::title_style())),
        Line::from(Span::styled(
            "  ──────────────────────────────────",
            theme::muted_style(),
        )),
        Line::from(Span::styled(
            format!(
                "  {} {}",
                if is_overdue {
                    theme::MARKER_OVERDUE
                } else {
                    theme::ARROW
                },
                action
            ),
            action_style,
        )),
    ]);
    frame.render_widget(next, chunks[1]);

    // Notes
    render_notes(frame, chunks[2], &data.notes);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" e", theme::key_style()),
        Span::styled("·edit  ", theme::muted_style()),
        Span::styled("n", theme::key_style()),
        Span::styled("·note  ", theme::muted_style()),
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

fn render_notes(frame: &mut Frame, area: Rect, notes: &[LeadNote]) {
    let mut lines = vec![
        Line::from(Span::styled("  NOTES", theme::title_style())),
        Line::from(Span::styled(
            "  ──────────────────────────────────",
            theme::muted_style(),
        )),
    ];

    if notes.is_empty() {
        lines.push(Line::from(Span::styled("  No notes", theme::muted_style())));
    } else {
        for note in notes {
            let date = &note.created_at[..10.min(note.created_at.len())];
            lines.push(Line::from(vec![
                Span::styled(format!("  {}  ", date), theme::muted_style()),
                Span::styled(&note.content, theme::label_style()),
            ]));
        }
    }

    let widget = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}
