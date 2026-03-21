//! Dashboard view — projects, pending changes, keybindings.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::tui::app::App;
use crate::tui::theme;

/// Render the dashboard.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let now = chrono::Local::now();
    let time_str = now.format("%H:%M %a").to_string();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    // ---- Header ----
    let left = " OS · Agema Labs";
    let right = format!("{}  {} ", app.user_name, time_str);
    let pad_len = (area.width as usize).saturating_sub(left.len() + right.len() + 2);
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" OS ", theme::title_style()),
        Span::styled("· Agema Labs", theme::muted_style()),
        Span::raw(" ".repeat(pad_len)),
        Span::styled(right, theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(header, chunks[0]);

    // ---- Content ----
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);

    render_projects(frame, content_chunks[0], app);
    render_right_panel(frame, content_chunks[1], app);

    // ---- Footer ----
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Enter", theme::key_style()),
        Span::styled("·open  ", theme::muted_style()),
        Span::styled("l", theme::key_style()),
        Span::styled("·status  ", theme::muted_style()),
        Span::styled("s", theme::key_style()),
        Span::styled("·search  ", theme::muted_style()),
        Span::styled("c", theme::key_style()),
        Span::styled("·changes  ", theme::muted_style()),
        Span::styled("/", theme::key_style()),
        Span::styled("·pipeline  ", theme::muted_style()),
        Span::styled("q", theme::key_style()),
        Span::styled("·quit", theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(footer, chunks[2]);
}

/// Render the project list.
fn render_projects(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines = vec![
        Line::from(Span::styled("  PROJECTS", theme::title_style())),
        Line::from(Span::styled(
            "  ─────────────────────────────────────",
            theme::muted_style(),
        )),
    ];

    if app.loading {
        lines.push(Line::from(Span::styled(
            "  Loading...",
            theme::muted_style(),
        )));
    } else if app.dashboard.projects.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No projects yet",
            theme::muted_style(),
        )));
    } else {
        for (i, project) in app.dashboard.projects.iter().enumerate() {
            let marker = if i == app.selected_index {
                theme::MARKER_ACTIVE
            } else if project.is_internal {
                theme::MARKER_INACTIVE
            } else {
                theme::MARKER_ACTIVE
            };

            let phase_display = format_phase(&project.phase);

            let style = if i == app.selected_index {
                theme::active_style()
            } else {
                theme::label_style()
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), style),
                Span::styled(format!("{:<20}", truncate(&project.name, 20)), style),
                Span::styled(format!("  {}", phase_display), theme::muted_style()),
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

/// Render the right panel — financials + pending changes.
fn render_right_panel(frame: &mut Frame, area: Rect, app: &App) {
    let fin = &app.dashboard.financials;
    let has_financials = fin.total_value > 0.0 || fin.total_invoiced > 0.0;

    let mut lines = Vec::new();

    if has_financials {
        lines.push(Line::from(Span::styled(
            "  FINANCIALS",
            theme::title_style(),
        )));
        lines.push(Line::from(Span::styled(
            "  ─────────────────────────────",
            theme::muted_style(),
        )));
        lines.push(Line::from(vec![
            Span::styled("  Invoiced    ", theme::muted_style()),
            Span::styled(
                format!("${:.0}K", fin.total_invoiced / 1000.0),
                theme::label_style(),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Outstanding ", theme::muted_style()),
            Span::styled(
                format!("${:.0}K", fin.total_outstanding / 1000.0),
                if fin.total_outstanding > 0.0 {
                    theme::warning_style()
                } else {
                    theme::success_style()
                },
            ),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "  PENDING CHANGES",
        theme::title_style(),
    )));
    lines.push(Line::from(Span::styled(
        "  ─────────────────────────────",
        theme::muted_style(),
    )));

    if app.dashboard.pending_changes_count > 0 {
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", theme::MARKER_OS), theme::warning_style()),
            Span::styled(
                format!(
                    "{} write-back{} awaiting review",
                    app.dashboard.pending_changes_count,
                    if app.dashboard.pending_changes_count == 1 {
                        ""
                    } else {
                        "s"
                    }
                ),
                theme::label_style(),
            ),
        ]));
    } else {
        lines.push(Line::from(Span::styled("  None", theme::muted_style())));
    }

    let widget = Paragraph::new(lines);
    frame.render_widget(widget, area);
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

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max - 1])
    } else {
        s.to_string()
    }
}
