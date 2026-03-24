//! Project view — files, tasks, decisions, repos, overview, keybindings.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::tui::app::App;
use crate::tui::theme;

/// Team member assigned to a project.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TeamMember {
    pub user_id: String,
    pub name: String,
    pub email: String,
    pub role: String, // "member" or "manager"
}

/// Project view data loaded from API.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ProjectData {
    pub name: String,
    pub description: Option<String>,
    pub phase: String,
    pub client_name: Option<String>,
    pub engagement_value: Option<f64>,
    pub files: Vec<FileEntry>,
    pub tasks: Vec<TaskEntry>,
    pub decisions: Vec<DecisionEntry>,
    pub repos: Vec<RepoEntry>,
    pub team: Vec<TeamMember>,
}

/// Linked GitHub repo.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RepoEntry {
    pub id: String,
    pub github_repo: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FileEntry {
    pub slug: String,
    pub category: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TaskEntry {
    pub title: String,
    pub status: String,
    pub assigned_to: Option<String>,
}

/// Decision entry for display.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DecisionEntry {
    pub id: String,
    pub title: String,
    pub status: String,
    pub resolution: Option<String>,
}

/// Fetch project data from the API.
pub async fn fetch(
    client: &crate::api_client::ApiClient,
    slug: &str,
) -> anyhow::Result<ProjectData> {
    let proj: serde_json::Value = client.get(&format!("/projects/{}", slug)).await?;
    let files_resp: serde_json::Value = client.get(&format!("/projects/{}/files", slug)).await?;
    let tasks_resp: serde_json::Value = client.get(&format!("/projects/{}/tasks", slug)).await?;
    let decisions_resp: serde_json::Value =
        client.get(&format!("/projects/{}/decisions", slug)).await?;
    let repos_resp: serde_json::Value = client
        .list_repos(slug)
        .await
        .unwrap_or(serde_json::json!({ "data": [] }));

    let files = files_resp["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|f| FileEntry {
                    slug: f["slug"].as_str().unwrap_or("?").to_string(),
                    category: f["category"].as_str().unwrap_or("other").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    let tasks = tasks_resp["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|t| TaskEntry {
                    title: t["title"].as_str().unwrap_or("?").to_string(),
                    status: t["status"].as_str().unwrap_or("open").to_string(),
                    assigned_to: t["assigned_to"].as_str().map(|s| s.to_string()),
                })
                .collect()
        })
        .unwrap_or_default();

    let decisions = decisions_resp["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|d| DecisionEntry {
                    id: d["id"].as_str().unwrap_or("").to_string(),
                    title: d["title"].as_str().unwrap_or("?").to_string(),
                    status: d["status"].as_str().unwrap_or("open").to_string(),
                    resolution: d["resolution"].as_str().map(|s| s.to_string()),
                })
                .collect()
        })
        .unwrap_or_default();

    let repos = repos_resp["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|r| RepoEntry {
                    id: r["id"].as_str().unwrap_or("").to_string(),
                    github_repo: r["github_repo"].as_str().unwrap_or("?").to_string(),
                    label: r["label"].as_str().map(|s| s.to_string()),
                })
                .collect()
        })
        .unwrap_or_default();

    let team = proj["data"]["team"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|m| TeamMember {
                    user_id: m["user_id"].as_str().unwrap_or("").to_string(),
                    name: m["name"].as_str().unwrap_or("Unknown").to_string(),
                    email: m["email"].as_str().unwrap_or("").to_string(),
                    role: m["role"].as_str().unwrap_or("member").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(ProjectData {
        name: proj["data"]["name"].as_str().unwrap_or("?").to_string(),
        description: proj["data"]["description"].as_str().map(|s| s.to_string()),
        phase: proj["data"]["phase"].as_str().unwrap_or("?").to_string(),
        client_name: proj["data"]["client_name"].as_str().map(|s| s.to_string()),
        engagement_value: proj["data"]["engagement_value"].as_f64(),
        files,
        tasks,
        decisions,
        repos,
        team,
    })
}

/// Render the project view.
pub fn render(frame: &mut Frame, area: Rect, app: &App, _slug: &str, data: &ProjectData) {
    let now = chrono::Local::now();
    let time_str = now.format("%H:%M %a").to_string();

    let has_input = app.input_mode.is_some();
    let has_description = data.description.is_some();
    let repos_height = if data.repos.is_empty() { 3 } else { (2 + data.repos.len()).min(6) as u16 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_input {
            vec![
                Constraint::Length(3),                                // header
                Constraint::Length(if has_description { 4 } else { 0 }), // overview
                Constraint::Min(6),                                   // files+tasks
                Constraint::Length(repos_height),                     // repos
                Constraint::Length(3),                                // footer
                Constraint::Length(3),                                // input
            ]
        } else {
            vec![
                Constraint::Length(3),                                // header
                Constraint::Length(if has_description { 4 } else { 0 }), // overview
                Constraint::Min(8),                                   // files+tasks
                Constraint::Length(repos_height),                     // repos
                Constraint::Length(3),                                // footer
                Constraint::Length(0),                                // input (hidden)
            ]
        })
        .split(area);

    // ---- Header: project name, client, phase, financials ----
    let phase = format_phase(&data.phase);
    let right = format!("{}  {} ", app.user_name, time_str);

    let title_left = format!(" PROJECT \u{00b7} {} ", data.name);
    let pad1 = (area.width as usize).saturating_sub(title_left.len() + right.len() + 2);

    let mut header_lines = vec![Line::from(vec![
        Span::styled(" PROJECT ", theme::title_style()),
        Span::styled(format!("\u{00b7} {} ", data.name), theme::label_style()),
        Span::raw(" ".repeat(pad1)),
        Span::styled(&right, theme::muted_style()),
    ])];

    // Second header line: Client + Phase + Value
    let mut detail_spans = vec![Span::raw("  ")];
    if let Some(ref client) = data.client_name {
        detail_spans.push(Span::styled("Client: ", theme::muted_style()));
        detail_spans.push(Span::styled(client.as_str(), theme::label_style()));
        detail_spans.push(Span::raw("   "));
    }
    detail_spans.push(Span::styled("Phase: ", theme::muted_style()));
    detail_spans.push(Span::styled(phase, theme::active_style()));
    if let Some(value) = data.engagement_value {
        detail_spans.push(Span::raw("   "));
        detail_spans.push(Span::styled("Value: ", theme::muted_style()));
        detail_spans.push(Span::styled(
            format_currency(value),
            theme::label_style(),
        ));
    }
    header_lines.push(Line::from(detail_spans));

    let header = Paragraph::new(header_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(header, chunks[0]);

    // ---- Overview: project description ----
    if has_description {
        if let Some(ref desc) = data.description {
            let overview = Paragraph::new(vec![
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(desc.as_str(), theme::label_style()),
                ]),
            ])
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(theme::muted_style()),
            );
            frame.render_widget(overview, chunks[1]);
        }
    }

    // ---- Content: files+decisions on left, tasks on right ----
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[2]);

    render_files_and_decisions(frame, content_chunks[0], &data.files, &data.decisions);
    render_tasks_and_team(frame, content_chunks[1], &data.tasks, &data.team);

    // ---- Repos: full width ----
    render_repos(frame, chunks[3], &data.repos);

    // ---- Input bar (when active) ----
    if has_input && chunks.len() > 5 {
        let label = match &app.input_mode {
            Some(crate::tui::app::InputMode::PushFile { .. }) => "File path",
            Some(crate::tui::app::InputMode::NewTask { .. }) => "Task title",
            Some(crate::tui::app::InputMode::NewDecision { .. }) => "Decision title",
            Some(crate::tui::app::InputMode::LinkRepo { .. }) => "Repo (owner/repo)",
            Some(crate::tui::app::InputMode::NewLead) => "Company name",
            Some(crate::tui::app::InputMode::NewProject) => "Project (name:slug)",
            Some(crate::tui::app::InputMode::AddLeadNote { .. }) => "Note",
            Some(crate::tui::app::InputMode::AddLeadContact { .. }) => "Contact (name:email)",
            Some(crate::tui::app::InputMode::ChatInput { .. }) => "Question",
            Some(crate::tui::app::InputMode::ResolveDecision { .. }) => "Resolution",
            Some(crate::tui::app::InputMode::AddTeamMember { .. }) => "Email address",
            None => "",
        };
        let input = Paragraph::new(Line::from(vec![
            Span::styled(format!("  {} > ", label), theme::active_style()),
            Span::styled(&app.input_buffer, theme::label_style()),
            Span::styled("\u{2588}", theme::active_style()),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::active_style()),
        );
        frame.render_widget(input, chunks[5]);
    }

    // ---- Footer ----
    let footer = if has_input {
        Paragraph::new(Line::from(vec![
            Span::styled(" Enter", theme::key_style()),
            Span::styled("\u{00b7}submit  ", theme::muted_style()),
            Span::styled("Esc", theme::key_style()),
            Span::styled("\u{00b7}cancel", theme::muted_style()),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled(" p", theme::key_style()),
            Span::styled("\u{00b7}push  ", theme::muted_style()),
            Span::styled("t", theme::key_style()),
            Span::styled("\u{00b7}task  ", theme::muted_style()),
            Span::styled("d", theme::key_style()),
            Span::styled("\u{00b7}decision  ", theme::muted_style()),
            Span::styled("r", theme::key_style()),
            Span::styled("\u{00b7}repo  ", theme::muted_style()),
            Span::styled("m", theme::key_style()),
            Span::styled("\u{00b7}team  ", theme::muted_style()),
            Span::styled("c", theme::key_style()),
            Span::styled("\u{00b7}chat  ", theme::muted_style()),
            Span::styled("b", theme::key_style()),
            Span::styled("\u{00b7}back", theme::muted_style()),
        ]))
    }
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(footer, chunks[4]);
}

/// Render files list with decisions below in the left column.
fn render_files_and_decisions(
    frame: &mut Frame,
    area: Rect,
    files: &[FileEntry],
    decisions: &[DecisionEntry],
) {
    let mut lines = vec![
        Line::from(Span::styled("  FILES", theme::title_style())),
        Line::from(Span::styled(
            "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
            theme::muted_style(),
        )),
    ];

    if files.is_empty() {
        lines.push(Line::from(Span::styled("  No files", theme::muted_style())));
    } else {
        for f in files {
            lines.push(Line::from(vec![
                Span::styled("  ", theme::label_style()),
                Span::styled(format!("{:<20}", f.slug), theme::label_style()),
                Span::styled(&f.category, theme::muted_style()),
            ]));
        }
    }

    // Decisions section below files
    let open_decisions: Vec<&DecisionEntry> =
        decisions.iter().filter(|d| d.status == "open").collect();

    if !open_decisions.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  DECISIONS",
            theme::title_style(),
        )));
        lines.push(Line::from(Span::styled(
            "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
            theme::muted_style(),
        )));

        for d in open_decisions {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", theme::MARKER_DECISION),
                    theme::warning_style(),
                ),
                Span::styled(&d.title, theme::warning_style()),
            ]));
        }
    }

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(widget, area);
}

/// Render tasks and team in the right column.
fn render_tasks_and_team(frame: &mut Frame, area: Rect, tasks: &[TaskEntry], team: &[TeamMember]) {
    let mut lines = vec![
        Line::from(Span::styled("  TASKS", theme::title_style())),
        Line::from(Span::styled(
            "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
            theme::muted_style(),
        )),
    ];

    if tasks.is_empty() {
        lines.push(Line::from(Span::styled("  No tasks", theme::muted_style())));
    } else {
        for t in tasks {
            let marker = match t.status.as_str() {
                "done" => theme::MARKER_DONE,
                "in_progress" => theme::MARKER_DECISION,
                _ => theme::MARKER_OPEN,
            };
            let style = match t.status.as_str() {
                "done" => theme::success_style(),
                "in_progress" => theme::warning_style(),
                _ => theme::label_style(),
            };

            let assigned = t
                .assigned_to
                .as_deref()
                .map(|a| format!("  @{}", a))
                .unwrap_or_default();

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), style),
                Span::styled(&t.title, style),
                Span::styled(assigned, theme::muted_style()),
            ]));
        }
    }

    // Team section — always shown
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  TEAM", theme::title_style())));
    lines.push(Line::from(Span::styled(
        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
        theme::muted_style(),
    )));
    if team.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", theme::MARKER_OVERDUE),
                theme::danger_style(),
            ),
            Span::styled("No users assigned", theme::danger_style()),
        ]));
        lines.push(Line::from(Span::styled(
            "  Press m to assign team",
            theme::muted_style(),
        )));
    } else {
        for m in team {
            let (marker, style) = if m.role == "manager" {
                ("\u{2605}", theme::active_style()) // star for PM
            } else {
                ("\u{25CB}", theme::label_style()) // circle for member
            };
            let role_label = if m.role == "manager" { " (PM)" } else { "" };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), style),
                Span::styled(&m.name, style),
                Span::styled(role_label, theme::muted_style()),
            ]));
        }
    }

    let widget = Paragraph::new(lines);
    frame.render_widget(widget, area);
}

/// Render repos in a full-width row.
fn render_repos(frame: &mut Frame, area: Rect, repos: &[RepoEntry]) {
    let mut lines = vec![
        Line::from(Span::styled("  REPOS", theme::title_style())),
    ];

    if repos.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No repos linked",
            theme::muted_style(),
        )));
    } else {
        for r in repos {
            let display = if let Some(ref label) = r.label {
                format!("  {} \u{00b7} {}", r.github_repo, label)
            } else {
                format!("  {}", r.github_repo)
            };
            lines.push(Line::from(Span::styled(display, theme::active_style())));
        }
    }

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(widget, area);
}

/// Push a file from a local path to the project via API.
pub async fn push_file(
    client: &crate::api_client::ApiClient,
    slug: &str,
    path_str: &str,
) -> anyhow::Result<String> {
    let path = std::path::Path::new(path_str.trim());
    if !path.exists() {
        anyhow::bail!("File not found: {}", path.display());
    }

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unnamed");

    let data = std::fs::read(path)?;

    let is_text = matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md" | "txt" | "csv" | "json" | "yaml" | "yml" | "toml")
    );

    if is_text {
        let text = String::from_utf8(data)?;
        let file_slug = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed");

        let body = serde_json::json!({
            "title": filename,
            "slug": file_slug,
            "category": "other",
            "content": text,
        });

        let _: serde_json::Value = client
            .post(&format!("/projects/{}/files", slug), &body)
            .await?;

        Ok(format!("Pushed {}", filename))
    } else {
        let mime = match path.extension().and_then(|e| e.to_str()) {
            Some("pdf") => "application/pdf",
            Some("png") => "image/png",
            Some("jpg" | "jpeg") => "image/jpeg",
            Some("zip") => "application/zip",
            _ => "application/octet-stream",
        };

        client
            .upload_file(
                &format!("/projects/{}/documents", slug),
                filename,
                mime,
                data,
            )
            .await?;

        Ok(format!("Uploaded {}", filename))
    }
}

/// Format a currency value as "$XK" or "$X.XM".
fn format_currency(amount: f64) -> String {
    if amount >= 1_000_000.0 {
        format!("${:.1}M", amount / 1_000_000.0)
    } else if amount >= 1_000.0 {
        format!("${:.0}K", amount / 1_000.0)
    } else {
        format!("${:.0}", amount)
    }
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
