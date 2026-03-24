//! Clients view — list all clients and client detail with engagements/projects.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::api_client::ApiClient;
use crate::tui::theme;

/// Client summary for list display.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ClientSummary {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub primary_contact: Option<String>,
    pub contact_email: Option<String>,
    pub engagement_count: i64,
    pub project_count: i64,
    pub total_value: Option<f64>,
    pub account_manager: Option<String>,
}

/// Engagement summary within a client.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EngagementSummary {
    pub id: String,
    pub name: String,
    pub outcome: Option<String>,
    pub value: Option<f64>,
}

/// Project summary within a client detail.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ClientProject {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub phase: String,
}

/// A file from a project within the client workspace.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WorkspaceFile {
    pub project_name: String,
    pub project_slug: String,
    pub file_title: String,
    pub file_slug: String,
    pub category: String,
}

/// Client detail data.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ClientDetailData {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub primary_contact: Option<String>,
    pub contact_email: Option<String>,
    pub notes: Option<String>,
    pub account_manager_name: Option<String>,
    pub account_manager_email: Option<String>,
    pub engagements: Vec<EngagementSummary>,
    pub projects: Vec<ClientProject>,
    pub workspace_files: Vec<WorkspaceFile>,
    pub show_workspace: bool,
    pub selected: usize,
}

/// State for the clients list view.
#[derive(Debug, Clone, Default)]
pub struct ClientsState {
    pub clients: Vec<ClientSummary>,
    pub selected: usize,
    pub loaded: bool,
    pub detail: Option<ClientDetailData>,
}

/// Fetch all clients from the API.
pub async fn fetch_clients(client: &ApiClient) -> anyhow::Result<Vec<ClientSummary>> {
    let resp: serde_json::Value = client.get("/clients?limit=100").await?;
    let clients = resp["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|v| ClientSummary {
                    id: v["id"].as_str().unwrap_or("").to_string(),
                    name: v["name"].as_str().unwrap_or("?").to_string(),
                    slug: v["slug"].as_str().unwrap_or("").to_string(),
                    primary_contact: v["primary_contact"].as_str().map(|s| s.to_string()),
                    contact_email: v["contact_email"].as_str().map(|s| s.to_string()),
                    engagement_count: v["engagement_count"].as_i64().unwrap_or(0),
                    project_count: v["project_count"].as_i64().unwrap_or(0),
                    total_value: v["total_value"].as_f64(),
                    account_manager: v["account_manager"].as_str().map(|s| s.to_string()),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(clients)
}

/// Fetch client detail from the API.
pub async fn fetch_client_detail(
    client: &ApiClient,
    slug: &str,
) -> anyhow::Result<ClientDetailData> {
    let resp: serde_json::Value = client.get(&format!("/clients/{}", slug)).await?;
    let data = &resp["data"];

    let engagements = data["engagements"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|e| EngagementSummary {
                    id: e["id"].as_str().unwrap_or("").to_string(),
                    name: e["name"].as_str().unwrap_or("?").to_string(),
                    outcome: e["outcome"].as_str().map(|s| s.to_string()),
                    value: e["value"].as_f64(),
                })
                .collect()
        })
        .unwrap_or_default();

    let projects: Vec<ClientProject> = data["projects"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|p| ClientProject {
                    id: p["id"].as_str().unwrap_or("").to_string(),
                    name: p["name"].as_str().unwrap_or("?").to_string(),
                    slug: p["slug"].as_str().unwrap_or("").to_string(),
                    description: p["description"].as_str().map(|s| s.to_string()),
                    phase: p["phase"].as_str().unwrap_or("discovery").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    let (account_manager_name, account_manager_email) = if data["account_manager"].is_object() {
        (
            data["account_manager"]["name"].as_str().map(|s| s.to_string()),
            data["account_manager"]["email"].as_str().map(|s| s.to_string()),
        )
    } else {
        (None, None)
    };

    // Fetch workspace files for each project.
    let mut workspace_files = Vec::new();
    for proj in &projects {
        let path = format!("/projects/{}/files", proj.slug);
        if let Ok(files_resp) = client
            .get::<serde_json::Value>(&path)
            .await
        {
            if let Some(arr) = files_resp["data"].as_array() {
                for f in arr {
                    workspace_files.push(WorkspaceFile {
                        project_name: proj.name.clone(),
                        project_slug: proj.slug.clone(),
                        file_title: f["title"].as_str().unwrap_or("?").to_string(),
                        file_slug: f["slug"].as_str().unwrap_or("").to_string(),
                        category: f["category"].as_str().unwrap_or("other").to_string(),
                    });
                }
            }
        }
    }

    Ok(ClientDetailData {
        id: data["id"].as_str().unwrap_or("").to_string(),
        name: data["name"].as_str().unwrap_or("?").to_string(),
        slug: data["slug"].as_str().unwrap_or("").to_string(),
        primary_contact: data["primary_contact"].as_str().map(|s| s.to_string()),
        contact_email: data["contact_email"].as_str().map(|s| s.to_string()),
        notes: data["notes"].as_str().map(|s| s.to_string()),
        account_manager_name,
        account_manager_email,
        engagements,
        projects,
        workspace_files,
        show_workspace: false,
        selected: 0,
    })
}

/// Render the clients list view.
pub fn render_list(frame: &mut Frame, area: Rect, state: &ClientsState, user_name: &str) {
    let now = chrono::Local::now();
    let time_str = now.format("%H:%M %a").to_string();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(8),   // content
            Constraint::Length(3), // footer
        ])
        .split(area);

    // Header
    let right = format!("{}  {} ", user_name, time_str);
    let left = " CLIENTS";
    let pad = (area.width as usize).saturating_sub(left.len() + right.len() + 2);
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" CLIENTS", theme::title_style()),
        Span::raw(" ".repeat(pad)),
        Span::styled(right, theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(header, chunks[0]);

    // Content
    // Calculate column widths proportionally based on terminal width
    let w = area.width as usize;
    let name_w = (w * 35 / 100).max(20); // ~35% for company name
    let divider = "\u{2500}".repeat(w.saturating_sub(4));

    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!("  {:<name_w$}", "Company"), theme::muted_style()),
            Span::styled("Proj ", theme::muted_style()),
            Span::styled("Value      ", theme::muted_style()),
            Span::styled("AM              ", theme::muted_style()),
            Span::styled("Contact", theme::muted_style()),
        ]),
        Line::from(Span::styled(format!("  {}", divider), theme::muted_style())),
    ];

    if !state.loaded {
        lines.push(Line::from(Span::styled(
            "  Loading...",
            theme::muted_style(),
        )));
    } else if state.clients.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No clients",
            theme::muted_style(),
        )));
    } else {
        for (i, c) in state.clients.iter().enumerate() {
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

            let value_str = c
                .total_value
                .map(format_currency)
                .unwrap_or_else(|| "\u{2014}".to_string());

            let am = c
                .account_manager
                .as_deref()
                .unwrap_or("\u{2014}");

            let contact = c
                .primary_contact
                .as_deref()
                .unwrap_or("\u{2014}");

            let name_col = name_w.saturating_sub(4); // account for marker + spaces
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), style),
                Span::styled(format!("{:<name_col$}", truncate(&c.name, name_col)), style),
                Span::styled(
                    format!("{:>4} ", c.project_count),
                    theme::muted_style(),
                ),
                Span::styled(format!("{:<11}", value_str), theme::label_style()),
                Span::styled(format!("{:<16}", truncate(am, 14)), theme::muted_style()),
                Span::styled(truncate(contact, 20), theme::muted_style()),
            ]));
        }
    }

    let content = Paragraph::new(lines);
    frame.render_widget(content, chunks[1]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Enter", theme::key_style()),
        Span::styled("\u{00b7}open  ", theme::muted_style()),
        Span::styled("b", theme::key_style()),
        Span::styled("\u{00b7}back", theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(footer, chunks[2]);
}

/// Render the client detail view.
pub fn render_detail(frame: &mut Frame, area: Rect, detail: &ClientDetailData, user_name: &str) {
    if detail.show_workspace {
        render_detail_workspace(frame, area, detail, user_name);
    } else {
        render_detail_projects(frame, area, detail, user_name);
    }
}

/// Render the client detail header lines (shared between both modes).
fn render_detail_header(area: Rect, detail: &ClientDetailData, user_name: &str, mode_label: &str) -> Vec<Line<'static>> {
    let now = chrono::Local::now();
    let time_str = now.format("%H:%M %a").to_string();
    let right = format!("{}  {} ", user_name.to_string(), time_str);
    let title = format!(" CLIENT \u{00b7} {} \u{00b7} {} ", detail.name, mode_label);
    let pad = (area.width as usize).saturating_sub(title.len() + right.len() + 2);

    let mut header_lines = vec![Line::from(vec![
        Span::styled(" CLIENT ".to_string(), theme::title_style()),
        Span::styled(format!("\u{00b7} {} \u{00b7} {} ", detail.name, mode_label), theme::label_style()),
        Span::raw(" ".repeat(pad)),
        Span::styled(right, theme::muted_style()),
    ])];

    let mut detail_spans = vec![Span::raw("  ")];
    if let Some(ref contact) = detail.primary_contact {
        detail_spans.push(Span::styled("Contact: ".to_string(), theme::muted_style()));
        detail_spans.push(Span::styled(contact.clone(), theme::label_style()));
    }
    if let Some(ref email) = detail.contact_email {
        detail_spans.push(Span::styled(
            format!(" \u{00b7} {}", email),
            theme::muted_style(),
        ));
    }
    if let Some(ref am) = detail.account_manager_name {
        detail_spans.push(Span::styled("  AM: ".to_string(), theme::muted_style()));
        detail_spans.push(Span::styled(am.clone(), theme::active_style()));
    }
    header_lines.push(Line::from(detail_spans));

    header_lines
}

/// Render client detail in projects mode (engagements + projects).
fn render_detail_projects(frame: &mut Frame, area: Rect, detail: &ClientDetailData, user_name: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // header
            Constraint::Min(4),   // engagements
            Constraint::Min(6),   // projects
            Constraint::Length(3), // footer
        ])
        .split(area);

    // Header
    let header_lines = render_detail_header(area, detail, user_name, "Projects");
    let header = Paragraph::new(header_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(header, chunks[0]);

    // Engagements
    let w = area.width as usize;
    let divider = "\u{2500}".repeat(w.saturating_sub(4));

    let mut eng_lines = vec![
        Line::from(Span::styled("  ENGAGEMENTS", theme::title_style())),
        Line::from(Span::styled(format!("  {}", divider), theme::muted_style())),
    ];

    if detail.engagements.is_empty() {
        eng_lines.push(Line::from(Span::styled(
            "  No engagements",
            theme::muted_style(),
        )));
    } else {
        for eng in &detail.engagements {
            let value_str = eng
                .value
                .map(format_currency)
                .unwrap_or_else(|| "\u{2014}".to_string());

            eng_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{:<30}", truncate(&eng.name, 30)), theme::label_style()),
                Span::styled(format!("{:<10}", value_str), theme::muted_style()),
            ]));
        }
    }

    let eng_widget = Paragraph::new(eng_lines).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(eng_widget, chunks[1]);

    // Projects
    let mut proj_lines = vec![
        Line::from(Span::styled("  PROJECTS", theme::title_style())),
        Line::from(Span::styled(format!("  {}", divider), theme::muted_style())),
    ];

    if detail.projects.is_empty() {
        proj_lines.push(Line::from(Span::styled(
            "  No projects",
            theme::muted_style(),
        )));
    } else {
        for (i, proj) in detail.projects.iter().enumerate() {
            let marker = if i == detail.selected {
                theme::MARKER_ACTIVE
            } else {
                theme::MARKER_OPEN
            };
            let style = if i == detail.selected {
                theme::active_style()
            } else {
                theme::label_style()
            };

            let phase = format_phase(&proj.phase);

            proj_lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), style),
                Span::styled(format!("{:<35}", truncate(&proj.name, 35)), style),
                Span::styled(phase.to_string(), theme::muted_style()),
            ]));
        }
    }

    let proj_widget = Paragraph::new(proj_lines);
    frame.render_widget(proj_widget, chunks[2]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" W", theme::key_style()),
        Span::styled("\u{00b7}workspace  ", theme::muted_style()),
        Span::styled("u", theme::key_style()),
        Span::styled("\u{00b7}add user  ", theme::muted_style()),
        Span::styled("Enter", theme::key_style()),
        Span::styled("\u{00b7}open project  ", theme::muted_style()),
        Span::styled("b", theme::key_style()),
        Span::styled("\u{00b7}back", theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(footer, chunks[3]);
}

/// Render client detail in workspace mode (files grouped by project).
fn render_detail_workspace(frame: &mut Frame, area: Rect, detail: &ClientDetailData, user_name: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // header
            Constraint::Min(6),   // workspace files
            Constraint::Length(3), // footer
        ])
        .split(area);

    // Header
    let header_lines = render_detail_header(area, detail, user_name, "Workspace");
    let header = Paragraph::new(header_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(header, chunks[0]);

    // Workspace files grouped by project
    let w = area.width as usize;
    let divider = "\u{2500}".repeat(w.saturating_sub(4));

    let mut lines = vec![
        Line::from(Span::styled("  WORKSPACE FILES", theme::title_style())),
        Line::from(Span::styled(format!("  {}", divider), theme::muted_style())),
    ];

    if detail.workspace_files.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No files",
            theme::muted_style(),
        )));
    } else {
        // Group files by project
        let mut current_project = String::new();
        for file in &detail.workspace_files {
            if file.project_name != current_project {
                if !current_project.is_empty() {
                    lines.push(Line::from(Span::raw("")));
                }
                lines.push(Line::from(Span::styled(
                    format!("  {}", file.project_name),
                    theme::label_style(),
                )));
                current_project = file.project_name.clone();
            }
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    format!("{:<25}", truncate(&file.file_title, 25)),
                    theme::muted_style(),
                ),
                Span::styled(file.category.clone(), theme::muted_style()),
            ]));
        }
    }

    let content = Paragraph::new(lines);
    frame.render_widget(content, chunks[1]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" W", theme::key_style()),
        Span::styled("\u{00b7}projects  ", theme::muted_style()),
        Span::styled("u", theme::key_style()),
        Span::styled("\u{00b7}add user  ", theme::muted_style()),
        Span::styled("b", theme::key_style()),
        Span::styled("\u{00b7}back", theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(footer, chunks[2]);
}

fn format_phase(phase: &str) -> &str {
    match phase {
        "discovery" => "Discovery",
        "in_progress" => "In Progress",
        "review" => "Review",
        "delivery" => "Delivery",
        "complete" => "Complete",
        "on_hold" => "On Hold",
        other => other,
    }
}

fn format_currency(amount: f64) -> String {
    if amount >= 1_000_000.0 {
        format!("${:.1}M", amount / 1_000_000.0)
    } else if amount >= 1_000.0 {
        format!("${:.0}K", amount / 1_000.0)
    } else {
        format!("${:.0}", amount)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}\u{2026}", &s[..max - 1])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clients_state_defaults() {
        let state = ClientsState::default();
        assert!(state.clients.is_empty());
        assert_eq!(state.selected, 0);
        assert!(!state.loaded);
    }

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string() {
        let result = truncate("a very long string here", 10);
        // 9 ASCII chars + 3-byte ellipsis = 12 bytes, but 10 display chars
        assert!(result.ends_with('\u{2026}'));
        assert_eq!(result.chars().count(), 10);
    }

    #[test]
    fn format_currency_values() {
        assert_eq!(format_currency(85000.0), "$85K");
        assert_eq!(format_currency(1_500_000.0), "$1.5M");
        assert_eq!(format_currency(500.0), "$500");
    }
}
