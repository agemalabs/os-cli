//! Lead detail view — contacts, notes, stage changes, delete.

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
    pub contacts: Vec<LeadContact>,
    /// Stage picker state.
    pub stage_picker: Option<StagePicker>,
    /// Confirmation dialog state.
    pub confirm_delete: bool,
}

/// Lead contact for display.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LeadContact {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role: Option<String>,
    pub is_primary: bool,
}

/// Stage picker overlay state.
#[derive(Debug, Clone)]
pub struct StagePicker {
    pub selected: usize,
    pub stages: Vec<String>,
}

impl StagePicker {
    /// Create a new stage picker with all available stages.
    pub fn new(current_stage: &str) -> Self {
        let stages = vec![
            "cold".to_string(),
            "warm".to_string(),
            "discovery".to_string(),
            "proposal_sent".to_string(),
            "negotiating".to_string(),
            "closed_won".to_string(),
            "closed_lost".to_string(),
        ];
        let selected = stages
            .iter()
            .position(|s| s == current_stage)
            .unwrap_or(0);
        Self { selected, stages }
    }

    /// Get the currently selected stage.
    pub fn current(&self) -> &str {
        &self.stages[self.selected]
    }
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

    let contacts = d["contacts"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|c| LeadContact {
                    id: c["id"].as_str().unwrap_or("").to_string(),
                    name: c["name"].as_str().unwrap_or("?").to_string(),
                    email: c["email"].as_str().map(|s| s.to_string()),
                    phone: c["phone"].as_str().map(|s| s.to_string()),
                    role: c["role"].as_str().map(|s| s.to_string()),
                    is_primary: c["is_primary"].as_bool().unwrap_or(false),
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
        contacts,
        stage_picker: None,
        confirm_delete: false,
    })
}

/// Render the lead detail view.
pub fn render(frame: &mut Frame, area: Rect, data: &LeadDetail) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // header
            Constraint::Length(4), // next action
            Constraint::Min(4),   // contacts + notes split
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
                value,
                format_stage(&data.stage),
                source
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
    let is_overdue = !action_date.is_empty();

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
                "  {} {} {}",
                if is_overdue {
                    theme::MARKER_OVERDUE
                } else {
                    theme::ARROW
                },
                action,
                if !action_date.is_empty() {
                    format!("({})", action_date)
                } else {
                    String::new()
                }
            ),
            action_style,
        )),
    ]);
    frame.render_widget(next, chunks[1]);

    // Content area — contacts on left, notes on right
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[2]);

    render_contacts(frame, content_chunks[0], &data.contacts);
    render_notes(frame, content_chunks[1], &data.notes);

    // Footer
    let footer = if data.confirm_delete {
        Paragraph::new(Line::from(vec![
            Span::styled(" Delete this lead? ", theme::danger_style()),
            Span::styled("y", theme::key_style()),
            Span::styled("·yes  ", theme::muted_style()),
            Span::styled("n", theme::key_style()),
            Span::styled("·no", theme::muted_style()),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled(" n", theme::key_style()),
            Span::styled("·note  ", theme::muted_style()),
            Span::styled("s", theme::key_style()),
            Span::styled("·stage  ", theme::muted_style()),
            Span::styled("a", theme::key_style()),
            Span::styled("·add contact  ", theme::muted_style()),
            Span::styled("d", theme::key_style()),
            Span::styled("·delete  ", theme::muted_style()),
            Span::styled("c", theme::key_style()),
            Span::styled("·chat  ", theme::muted_style()),
            Span::styled("b", theme::key_style()),
            Span::styled("·back", theme::muted_style()),
        ]))
    }
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(footer, chunks[3]);

    // Render stage picker overlay if active
    if let Some(picker) = &data.stage_picker {
        render_stage_picker(frame, area, picker);
    }
}

fn render_contacts(frame: &mut Frame, area: Rect, contacts: &[LeadContact]) {
    let mut lines = vec![
        Line::from(Span::styled("  CONTACTS", theme::title_style())),
        Line::from(Span::styled(
            "  ──────────────────────",
            theme::muted_style(),
        )),
    ];

    if contacts.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No contacts",
            theme::muted_style(),
        )));
    } else {
        for c in contacts {
            let primary_marker = if c.is_primary { " [primary]" } else { "" };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", theme::MARKER_ACTIVE), theme::active_style()),
                Span::styled(&c.name, theme::label_style()),
                Span::styled(primary_marker, theme::success_style()),
            ]));
            if let Some(email) = &c.email {
                lines.push(Line::from(Span::styled(
                    format!("    {}", email),
                    theme::muted_style(),
                )));
            }
            if let Some(role) = &c.role {
                lines.push(Line::from(Span::styled(
                    format!("    {}", role),
                    theme::muted_style(),
                )));
            }
        }
    }

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(widget, area);
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

fn render_stage_picker(frame: &mut Frame, area: Rect, picker: &StagePicker) {
    // Center a small overlay
    let width = 30u16;
    let height = (picker.stages.len() as u16) + 4;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(x, y, width, height);

    let mut lines = vec![
        Line::from(Span::styled(" Select Stage", theme::title_style())),
        Line::from(Span::styled(
            " ──────────────────────",
            theme::muted_style(),
        )),
    ];

    for (i, stage) in picker.stages.iter().enumerate() {
        let is_selected = i == picker.selected;
        let marker = if is_selected {
            theme::MARKER_ACTIVE
        } else {
            theme::MARKER_OPEN
        };
        let style = if is_selected {
            theme::active_style()
        } else {
            theme::label_style()
        };
        lines.push(Line::from(Span::styled(
            format!(" {} {}", marker, format_stage(stage)),
            style,
        )));
    }

    lines.push(Line::from(vec![
        Span::styled(" Enter", theme::key_style()),
        Span::styled("·select  ", theme::muted_style()),
        Span::styled("Esc", theme::key_style()),
        Span::styled("·cancel", theme::muted_style()),
    ]));

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::active_style()),
    );
    frame.render_widget(widget, popup_area);
}

fn format_stage(stage: &str) -> &str {
    match stage {
        "cold" => "Cold",
        "warm" => "Warm",
        "discovery" => "Discovery",
        "proposal_sent" => "Proposal Sent",
        "negotiating" => "Negotiating",
        "closed_won" => "Closed Won",
        "closed_lost" => "Closed Lost",
        other => other,
    }
}
