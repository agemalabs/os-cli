//! Status/Loop view — AI-generated briefing + structured data.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::tui::theme;

/// Which section is focused in the status view.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum StatusSection {
    #[default]
    Tasks,
    Decisions,
}

/// Status view data loaded from the API.
#[derive(Debug, Clone, Default)]
pub struct StatusData {
    pub summary: String,
    pub tasks: Vec<StatusTask>,
    pub overdue: Vec<StatusTask>,
    pub decisions: Vec<StatusDecision>,
    pub pending_changes: usize,
    pub selected_section: StatusSection,
    pub selected_index: usize,
}

#[derive(Debug, Clone)]
pub struct StatusTask {
    pub id: String,
    pub title: String,
    pub status: String,
    pub due: Option<String>,
    pub project_slug: String,
}

#[derive(Debug, Clone)]
pub struct StatusDecision {
    pub id: String,
    pub title: String,
    pub project: String,
}

/// Fetch status data from the API.
pub async fn fetch(client: &crate::api_client::ApiClient) -> anyhow::Result<StatusData> {
    let resp: serde_json::Value = client.get("/status").await?;
    let data = &resp["data"];

    let summary = data["generated_summary"]
        .as_str()
        .unwrap_or("No summary available.")
        .to_string();

    let tasks = data["my_tasks"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|t| StatusTask {
                    id: t["id"].as_str().unwrap_or("").to_string(),
                    title: t["title"].as_str().unwrap_or("?").to_string(),
                    status: t["status"].as_str().unwrap_or("open").to_string(),
                    due: t["due_date"].as_str().map(|s| s.to_string()),
                    project_slug: t["project_slug"].as_str().unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    let overdue = data["overdue_tasks"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|t| StatusTask {
                    id: t["id"].as_str().unwrap_or("").to_string(),
                    title: t["title"].as_str().unwrap_or("?").to_string(),
                    status: t["status"].as_str().unwrap_or("open").to_string(),
                    due: t["due_date"].as_str().map(|s| s.to_string()),
                    project_slug: t["project_slug"].as_str().unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    let decisions = data["open_decisions"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|d| StatusDecision {
                    id: d["id"].as_str().unwrap_or("").to_string(),
                    title: d["title"].as_str().unwrap_or("?").to_string(),
                    project: d["project_slug"]
                        .as_str()
                        .or_else(|| d["project"].as_str())
                        .unwrap_or("?")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    let pending_changes = data["pending_changes"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);

    Ok(StatusData {
        summary,
        tasks,
        overdue,
        decisions,
        pending_changes,
        selected_section: StatusSection::default(),
        selected_index: 0,
    })
}

/// Render the status view.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    data: &StatusData,
    user_name: &str,
    input_active: bool,
    input_buffer: &str,
) {
    let now = chrono::Local::now();
    let time_str = now.format("%H:%M %a").to_string();
    let date_str = now.format("%a %b %-d").to_string();

    let mut constraints = vec![
        Constraint::Length(3), // header
        Constraint::Length(6), // AI summary
        Constraint::Min(6),   // content
    ];
    if input_active {
        constraints.push(Constraint::Length(3)); // input bar
    }
    constraints.push(Constraint::Length(3)); // footer

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    // Header
    let left = format!(" STATUS · {} · {}", user_name, date_str);
    let right = format!("{} ", time_str);
    let pad_len = (area.width as usize).saturating_sub(left.len() + right.len() + 2);
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" STATUS ", theme::title_style()),
        Span::styled(
            format!("· {} · {}", user_name, date_str),
            theme::muted_style(),
        ),
        Span::raw(" ".repeat(pad_len)),
        Span::styled(right, theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(header, chunks[0]);

    // AI summary paragraph
    let summary = Paragraph::new(format!("\n  {}", data.summary))
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(theme::muted_style()),
        );
    frame.render_widget(summary, chunks[1]);

    // Content — tasks on left, decisions + changes on right
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[2]);

    render_tasks(
        frame,
        content_chunks[0],
        &data.tasks,
        &data.overdue,
        data.selected_section == StatusSection::Tasks,
        data.selected_index,
    );
    render_right(
        frame,
        content_chunks[1],
        &data.decisions,
        data.pending_changes,
        data.selected_section == StatusSection::Decisions,
        data.selected_index,
    );

    // Input bar (when resolving a decision)
    let footer_chunk_idx = if input_active {
        let input_idx = 3;
        let input_bar = Paragraph::new(Line::from(vec![
            Span::styled("  Resolution > ", theme::active_style()),
            Span::styled(input_buffer, theme::label_style()),
            Span::styled("\u{2588}", theme::active_style()),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::active_style()),
        );
        frame.render_widget(input_bar, chunks[input_idx]);
        input_idx + 1
    } else {
        3
    };

    // Footer
    let footer = if input_active {
        Paragraph::new(Line::from(vec![
            Span::styled(" Enter", theme::key_style()),
            Span::styled("·submit  ", theme::muted_style()),
            Span::styled("Esc", theme::key_style()),
            Span::styled("·cancel", theme::muted_style()),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled(" j/k", theme::key_style()),
            Span::styled("·navigate  ", theme::muted_style()),
            Span::styled("tab", theme::key_style()),
            Span::styled("·section  ", theme::muted_style()),
            Span::styled("\u{21b5}", theme::key_style()),
            Span::styled("·toggle status  ", theme::muted_style()),
            Span::styled("r", theme::key_style()),
            Span::styled("·resolve  ", theme::muted_style()),
            Span::styled("c", theme::key_style()),
            Span::styled("·chat  ", theme::muted_style()),
            Span::styled("b", theme::key_style()),
            Span::styled("·back  ", theme::muted_style()),
            Span::styled("q", theme::key_style()),
            Span::styled("·quit", theme::muted_style()),
        ]))
    }
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(footer, chunks[footer_chunk_idx]);
}

fn render_tasks(
    frame: &mut Frame,
    area: Rect,
    tasks: &[StatusTask],
    overdue: &[StatusTask],
    is_active_section: bool,
    selected_index: usize,
) {
    let mut lines = vec![
        Line::from(Span::styled("  YOUR TASKS", theme::title_style())),
        Line::from(Span::styled(
            "  ─────────────────────────────",
            theme::muted_style(),
        )),
    ];

    // Combined list: overdue first, then regular tasks
    let mut item_index: usize = 0;

    if !overdue.is_empty() {
        for t in overdue {
            let is_selected = is_active_section && item_index == selected_index;
            let sel_marker = if is_selected {
                theme::MARKER_ACTIVE
            } else {
                theme::MARKER_OVERDUE
            };
            let style = if is_selected {
                theme::active_style()
            } else {
                theme::danger_style()
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", sel_marker), style),
                Span::styled(&t.title, style),
                Span::styled("  overdue", style),
            ]));
            item_index += 1;
        }
    }

    if tasks.is_empty() && overdue.is_empty() {
        lines.push(Line::from(Span::styled(
            "  All clear",
            theme::muted_style(),
        )));
    } else {
        for t in tasks {
            let is_selected = is_active_section && item_index == selected_index;
            let marker = if is_selected {
                theme::MARKER_ACTIVE
            } else {
                match t.status.as_str() {
                    "done" => theme::MARKER_DONE,
                    "in_progress" => theme::MARKER_DECISION,
                    _ => theme::MARKER_OPEN,
                }
            };
            let due_text = t
                .due
                .as_deref()
                .map(|d| format!("  due {}", d))
                .unwrap_or_default();

            let style = if is_selected {
                theme::active_style()
            } else {
                theme::label_style()
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), style),
                Span::styled(&t.title, style),
                Span::styled(
                    due_text,
                    if is_selected {
                        theme::active_style()
                    } else {
                        theme::muted_style()
                    },
                ),
            ]));
            item_index += 1;
        }
    }

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(widget, area);
}

fn render_right(
    frame: &mut Frame,
    area: Rect,
    decisions: &[StatusDecision],
    pending_changes: usize,
    is_active_section: bool,
    selected_index: usize,
) {
    let mut lines = vec![
        Line::from(Span::styled("  DECISIONS NEEDED", theme::title_style())),
        Line::from(Span::styled(
            "  ─────────────────────────────",
            theme::muted_style(),
        )),
    ];

    if decisions.is_empty() {
        lines.push(Line::from(Span::styled("  None", theme::muted_style())));
    } else {
        for (i, d) in decisions.iter().enumerate() {
            let is_selected = is_active_section && i == selected_index;
            let marker = if is_selected {
                theme::MARKER_ACTIVE
            } else {
                theme::MARKER_DECISION
            };
            let style = if is_selected {
                theme::active_style()
            } else {
                theme::warning_style()
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), style),
                Span::styled(
                    &d.project,
                    if is_selected {
                        theme::active_style()
                    } else {
                        theme::muted_style()
                    },
                ),
                Span::styled(
                    format!("  {}", d.title),
                    if is_selected {
                        theme::active_style()
                    } else {
                        theme::label_style()
                    },
                ),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  PENDING CHANGES",
        theme::title_style(),
    )));
    lines.push(Line::from(Span::styled(
        "  ─────────────────────────────",
        theme::muted_style(),
    )));

    if pending_changes > 0 {
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", theme::MARKER_OS), theme::warning_style()),
            Span::styled(
                format!("{} awaiting review", pending_changes),
                theme::label_style(),
            ),
        ]));
    } else {
        lines.push(Line::from(Span::styled("  None", theme::muted_style())));
    }

    let widget = Paragraph::new(lines);
    frame.render_widget(widget, area);
}
