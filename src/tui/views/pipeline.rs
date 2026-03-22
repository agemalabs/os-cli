//! Pipeline view — leads list with stage, value, next action, search/filter.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::tui::theme;

/// Pipeline view state.
#[derive(Debug, Clone, Default)]
pub struct PipelineState {
    pub leads: Vec<LeadSummary>,
    pub selected: usize,
    pub loaded: bool,
    /// Active filter text (when `/` is pressed).
    pub filter: Option<String>,
    /// Whether we are in filter input mode.
    pub filtering: bool,
    /// Confirm delete state.
    pub confirm_delete: bool,
}

/// Lead summary for list display.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LeadSummary {
    pub id: String,
    pub company_name: String,
    pub contact_name: Option<String>,
    pub estimated_value: Option<String>,
    pub stage: String,
    pub next_action: Option<String>,
    pub next_action_date: Option<String>,
}

impl PipelineState {
    /// Get leads filtered by the current filter text.
    pub fn filtered_leads(&self) -> Vec<&LeadSummary> {
        match &self.filter {
            Some(f) if !f.is_empty() => {
                let f_lower = f.to_lowercase();
                self.leads
                    .iter()
                    .filter(|l| l.company_name.to_lowercase().contains(&f_lower))
                    .collect()
            }
            _ => self.leads.iter().collect(),
        }
    }
}

/// Fetch leads from the API.
pub async fn fetch(client: &crate::api_client::ApiClient) -> anyhow::Result<Vec<LeadSummary>> {
    let resp: serde_json::Value = client.get("/leads").await?;
    let leads = resp["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|l| LeadSummary {
                    id: l["id"].as_str().unwrap_or("").to_string(),
                    company_name: l["company_name"].as_str().unwrap_or("?").to_string(),
                    contact_name: l["contact_name"].as_str().map(|s| s.to_string()),
                    estimated_value: l["estimated_value"]
                        .as_f64()
                        .map(|v| format!("${:.0}K", v / 1000.0)),
                    stage: l["stage"].as_str().unwrap_or("cold").to_string(),
                    next_action: l["next_action"].as_str().map(|s| s.to_string()),
                    next_action_date: l["next_action_date"].as_str().map(|s| s.to_string()),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(leads)
}

/// Render the pipeline view.
pub fn render(frame: &mut Frame, area: Rect, state: &PipelineState) {
    let has_filter = state.filtering;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_filter {
            vec![
                Constraint::Length(3),
                Constraint::Min(6),
                Constraint::Length(3),
                Constraint::Length(3),
            ]
        } else {
            vec![
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(3),
                Constraint::Length(0),
            ]
        })
        .split(area);

    // Header
    let filtered = state.filtered_leads();
    let total: f64 = filtered
        .iter()
        .filter_map(|l| {
            l.estimated_value
                .as_ref()
                .and_then(|v| {
                    v.trim_start_matches('$')
                        .trim_end_matches('K')
                        .parse::<f64>()
                        .ok()
                })
                .map(|v| v * 1000.0)
        })
        .sum();

    let header = Paragraph::new(Line::from(vec![
        Span::styled(" PIPELINE", theme::title_style()),
        Span::styled(
            format!(
                "  {} leads  ${:.0}K total",
                filtered.len(),
                total / 1000.0
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

    // Lead list
    render_leads(frame, chunks[1], state);

    // Footer
    let footer = if state.confirm_delete {
        Paragraph::new(Line::from(vec![
            Span::styled(" Delete this lead? ", theme::danger_style()),
            Span::styled("y", theme::key_style()),
            Span::styled("·yes  ", theme::muted_style()),
            Span::styled("n", theme::key_style()),
            Span::styled("·no", theme::muted_style()),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled(" Enter", theme::key_style()),
            Span::styled("·open  ", theme::muted_style()),
            Span::styled("n", theme::key_style()),
            Span::styled("·new lead  ", theme::muted_style()),
            Span::styled("d", theme::key_style()),
            Span::styled("·delete  ", theme::muted_style()),
            Span::styled("/", theme::key_style()),
            Span::styled("·filter  ", theme::muted_style()),
            Span::styled("j/k", theme::key_style()),
            Span::styled("·navigate  ", theme::muted_style()),
            Span::styled("b", theme::key_style()),
            Span::styled("·back", theme::muted_style()),
        ]))
    }
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(footer, chunks[2]);

    // Filter input bar
    if has_filter && chunks.len() > 3 {
        let filter_text = state.filter.as_deref().unwrap_or("");
        let input = Paragraph::new(Line::from(vec![
            Span::styled("  Filter > ", theme::active_style()),
            Span::styled(filter_text, theme::label_style()),
            Span::styled("█", theme::active_style()),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::active_style()),
        );
        frame.render_widget(input, chunks[3]);
    }
}

fn render_leads(frame: &mut Frame, area: Rect, state: &PipelineState) {
    let filtered = state.filtered_leads();

    let mut lines = vec![
        Line::from(vec![
            Span::styled("  Company", theme::muted_style()),
            Span::raw("              "),
            Span::styled("Value", theme::muted_style()),
            Span::raw("     "),
            Span::styled("Stage", theme::muted_style()),
            Span::raw("            "),
            Span::styled("Next Action", theme::muted_style()),
        ]),
        Line::from(Span::styled(
            "  ──────────────────────────────────────────────────────────────────",
            theme::muted_style(),
        )),
    ];

    if !state.loaded {
        lines.push(Line::from(Span::styled(
            "  Loading...",
            theme::muted_style(),
        )));
    } else if filtered.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No leads",
            theme::muted_style(),
        )));
    } else {
        for (i, lead) in filtered.iter().enumerate() {
            let is_selected = i == state.selected;
            let marker = match lead.stage.as_str() {
                "closed_won" => theme::MARKER_DONE,
                "closed_lost" => theme::MARKER_OPEN,
                _ => {
                    if is_selected {
                        theme::MARKER_ACTIVE
                    } else {
                        theme::MARKER_OPEN
                    }
                }
            };

            let style = if is_selected {
                theme::active_style()
            } else {
                theme::label_style()
            };

            let value = lead.estimated_value.as_deref().unwrap_or("—");
            let stage = format_stage(&lead.stage);
            let action = lead.next_action.as_deref().unwrap_or("");

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), style),
                Span::styled(format!("{:<18}", truncate(&lead.company_name, 18)), style),
                Span::styled(format!("{:<9}", value), theme::muted_style()),
                Span::styled(format!("{:<15}", stage), stage_style(&lead.stage)),
                Span::styled(truncate(action, 20), theme::muted_style()),
            ]));
        }
    }

    let widget = Paragraph::new(lines);
    frame.render_widget(widget, area);
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

fn stage_style(stage: &str) -> ratatui::style::Style {
    match stage {
        "closed_won" => theme::success_style(),
        "closed_lost" => theme::danger_style(),
        "negotiating" | "proposal_sent" => theme::warning_style(),
        _ => theme::muted_style(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max - 1])
    } else {
        s.to_string()
    }
}
