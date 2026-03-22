//! Project view — files, tasks, phase, keybindings.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::tui::app::App;
use crate::tui::theme;

/// Project view data loaded from API.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ProjectData {
    pub name: String,
    pub description: Option<String>,
    pub phase: String,
    pub files: Vec<FileEntry>,
    pub tasks: Vec<TaskEntry>,
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

/// Fetch project data from the API.
pub async fn fetch(
    client: &crate::api_client::ApiClient,
    slug: &str,
) -> anyhow::Result<ProjectData> {
    let proj: serde_json::Value = client.get(&format!("/projects/{}", slug)).await?;
    let files_resp: serde_json::Value = client.get(&format!("/projects/{}/files", slug)).await?;
    let tasks_resp: serde_json::Value = client.get(&format!("/projects/{}/tasks", slug)).await?;

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

    Ok(ProjectData {
        name: proj["data"]["name"].as_str().unwrap_or("?").to_string(),
        description: proj["data"]["description"].as_str().map(|s| s.to_string()),
        phase: proj["data"]["phase"].as_str().unwrap_or("?").to_string(),
        files,
        tasks,
    })
}

/// Render the project view.
pub fn render(frame: &mut Frame, area: Rect, app: &App, _slug: &str, data: &ProjectData) {
    let now = chrono::Local::now();
    let time_str = now.format("%H:%M %a").to_string();

    let has_input = app.input_mode.is_some();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_input {
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
    let phase = format_phase(&data.phase);
    let left = format!(" PROJECT · {} · {}", data.name, phase);
    let right = format!("{} ", time_str);
    let pad_len = (area.width as usize).saturating_sub(left.len() + right.len() + 2);
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" PROJECT ", theme::title_style()),
        Span::styled(format!("· {} · {}", data.name, phase), theme::muted_style()),
        Span::raw(" ".repeat(pad_len)),
        Span::styled(right, theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(header, chunks[0]);

    // Content — files on left, tasks on right
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[1]);

    render_files(frame, content_chunks[0], &data.files);
    render_tasks(frame, content_chunks[1], &data.tasks);

    // Input bar (when active)
    if has_input && chunks.len() > 3 {
        let label = match &app.input_mode {
            Some(crate::tui::app::InputMode::PushFile { .. }) => "File path",
            Some(crate::tui::app::InputMode::NewTask { .. }) => "Task title",
            Some(crate::tui::app::InputMode::NewLead) => "Company name",
            None => "",
        };
        let input = Paragraph::new(Line::from(vec![
            Span::styled(format!("  {} > ", label), theme::active_style()),
            Span::styled(&app.input_buffer, theme::label_style()),
            Span::styled("█", theme::active_style()),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::active_style()),
        );
        frame.render_widget(input, chunks[3]);
    }

    // Footer
    let footer = if has_input {
        Paragraph::new(Line::from(vec![
            Span::styled(" Enter", theme::key_style()),
            Span::styled("·submit  ", theme::muted_style()),
            Span::styled("Esc", theme::key_style()),
            Span::styled("·cancel", theme::muted_style()),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled(" t", theme::key_style()),
            Span::styled("·task  ", theme::muted_style()),
            Span::styled("d", theme::key_style()),
            Span::styled("·decision  ", theme::muted_style()),
            Span::styled("p", theme::key_style()),
            Span::styled("·push file  ", theme::muted_style()),
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
}

fn render_files(frame: &mut Frame, area: Rect, files: &[FileEntry]) {
    let mut lines = vec![
        Line::from(Span::styled("  FILES", theme::title_style())),
        Line::from(Span::styled(
            "  ──────────────────────────",
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

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(widget, area);
}

fn render_tasks(frame: &mut Frame, area: Rect, tasks: &[TaskEntry]) {
    let mut lines = vec![
        Line::from(Span::styled("  TASKS", theme::title_style())),
        Line::from(Span::styled(
            "  ──────────────────────────────────",
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

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), style),
                Span::styled(&t.title, style),
            ]));
        }
    }

    let widget = Paragraph::new(lines);
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
