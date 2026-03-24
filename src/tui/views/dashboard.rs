//! Dashboard view — projects, pending changes, activity, revenue chart, keybindings.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph};
use ratatui::Frame;

use crate::tui::app::App;
use crate::tui::data::RevenueChart;
use crate::tui::theme;

/// Render the dashboard.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let now = chrono::Local::now();
    let time_str = now.format("%H:%M %a").to_string();

    let has_chart = !app.dashboard.revenue_chart.week_labels.is_empty();
    let chart_height = if has_chart { 10 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_chart {
            vec![
                Constraint::Length(3),
                Constraint::Length(chart_height),
                Constraint::Min(8),
                Constraint::Length(3),
            ]
        } else {
            vec![
                Constraint::Length(3),
                Constraint::Length(0),
                Constraint::Min(8),
                Constraint::Length(3),
            ]
        })
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

    // ---- Revenue Chart ----
    if has_chart {
        render_revenue_chart(frame, chunks[1], &app.dashboard.revenue_chart);
    }

    // ---- Content ----
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    render_projects(frame, content_chunks[0], app);
    render_right_panel(frame, content_chunks[1], app);

    // ---- Input bar ----
    if app.input_mode.is_some() {
        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(6),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(area);

        let label = match &app.input_mode {
            Some(crate::tui::app::InputMode::NewProject) => "Project (name:slug)",
            _ => "Input",
        };
        let input_bar = Paragraph::new(Line::from(vec![
            Span::styled(format!("  {} > ", label), theme::active_style()),
            Span::styled(&app.input_buffer, theme::label_style()),
            Span::styled("█", theme::active_style()),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::active_style()),
        );
        frame.render_widget(input_bar, input_chunks[3]);
    }

    // ---- Footer ----
    let footer = if app.input_mode.is_some() {
        Paragraph::new(Line::from(vec![
            Span::styled(" Enter", theme::key_style()),
            Span::styled("·submit  ", theme::muted_style()),
            Span::styled("Esc", theme::key_style()),
            Span::styled("·cancel", theme::muted_style()),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled(" Enter", theme::key_style()),
            Span::styled("·open  ", theme::muted_style()),
            Span::styled("n", theme::key_style()),
            Span::styled("·new project  ", theme::muted_style()),
            Span::styled("w", theme::key_style()),
            Span::styled(
                if app.activity_days == 1 {
                    "·week  "
                } else {
                    "·today  "
                },
                theme::muted_style(),
            ),
            Span::styled("l", theme::key_style()),
            Span::styled("·status  ", theme::muted_style()),
            Span::styled("s", theme::key_style()),
            Span::styled("·search  ", theme::muted_style()),
            Span::styled("c", theme::key_style()),
            Span::styled("·chat  ", theme::muted_style()),
            Span::styled("x", theme::key_style()),
            Span::styled("·changes  ", theme::muted_style()),
            Span::styled("C", theme::key_style()),
            Span::styled("·clients  ", theme::muted_style()),
            Span::styled("/", theme::key_style()),
            Span::styled("·pipeline  ", theme::muted_style()),
            Span::styled("q", theme::key_style()),
            Span::styled("·quit", theme::muted_style()),
        ]))
    }
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(footer, chunks[3]);
}

/// Render the revenue chart using ratatui's Chart widget with three data series:
/// Paid (green), Invoiced (blue), and Projected (muted/dotted).
fn render_revenue_chart(frame: &mut Frame, area: Rect, chart_data: &RevenueChart) {
    if chart_data.week_labels.is_empty() || area.height < 3 {
        return;
    }

    let num_weeks = chart_data.week_labels.len();

    // Convert data to (f64, f64) points for Chart widget
    let paid_points: Vec<(f64, f64)> = chart_data
        .paid
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f64, *v))
        .collect();

    let invoiced_points: Vec<(f64, f64)> = chart_data
        .invoiced
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f64, *v))
        .collect();

    let projected_points: Vec<(f64, f64)> = chart_data
        .projected
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f64, *v))
        .collect();

    let max_val = paid_points
        .iter()
        .chain(invoiced_points.iter())
        .chain(projected_points.iter())
        .map(|(_, v)| *v)
        .fold(0.0_f64, f64::max)
        .max(1.0);

    let datasets = vec![
        Dataset::default()
            .name("Paid")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(theme::success_style())
            .data(&paid_points),
        Dataset::default()
            .name("Invoiced")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(theme::active_style())
            .data(&invoiced_points),
        Dataset::default()
            .name("Projected")
            .marker(Marker::Dot)
            .graph_type(GraphType::Line)
            .style(theme::muted_style())
            .data(&projected_points),
    ];

    // X-axis labels — show month names at transitions
    let mut x_labels: Vec<Span> = Vec::new();
    let mut last_month = String::new();
    for (i, week_str) in chart_data.week_labels.iter().enumerate() {
        let month = chrono::NaiveDate::parse_from_str(week_str, "%Y-%m-%d")
            .map(|d| d.format("%b").to_string())
            .unwrap_or_default();
        if month != last_month {
            // Only show at first, middle-ish, and last positions to avoid clutter
            if i == 0 || i == num_weeks / 2 || i == num_weeks - 1 {
                x_labels.push(Span::styled(month.clone(), theme::muted_style()));
            }
            last_month = month;
        }
    }
    // Ensure we have at least start and end labels
    if x_labels.is_empty() {
        x_labels.push(Span::raw(""));
    }

    let y_max = max_val * 1.1;
    let y_labels = vec![
        Span::raw("$0"),
        Span::raw(format_currency(y_max / 2.0)),
        Span::raw(format_currency(y_max)),
    ];

    let chart_widget = Chart::new(datasets)
        .block(
            Block::default()
                .title(Span::styled(
                    format!(
                        " REVENUE  Total: {}  Avg: {}/wk ",
                        format_currency(chart_data.total_90d),
                        format_currency(chart_data.avg_weekly),
                    ),
                    theme::title_style(),
                ))
                .borders(Borders::BOTTOM)
                .border_style(theme::muted_style()),
        )
        .x_axis(
            Axis::default()
                .bounds([0.0, (num_weeks as f64 - 1.0).max(1.0)])
                .labels(x_labels),
        )
        .y_axis(Axis::default().bounds([0.0, y_max]).labels(y_labels));

    frame.render_widget(chart_widget, area);
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

            let team_warning = if project.team_count == 0 && !project.is_internal {
                Span::styled("  ⚠ no team", theme::danger_style())
            } else {
                Span::raw("")
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), style),
                Span::styled(format!("{:<20}", truncate(&project.name, 20)), style),
                Span::styled(format!("  {}", phase_display), theme::muted_style()),
                team_warning,
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

/// Render the right panel — financials + pending changes + activity.
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

        // Total engagement value
        if fin.total_value > 0.0 {
            lines.push(Line::from(vec![
                Span::styled("  Total Value   ", theme::muted_style()),
                Span::styled(
                    format_currency(fin.total_value),
                    theme::label_style(),
                ),
            ]));
        }

        // Invoiced with progress bar
        let invoiced_bar = progress_bar(fin.total_invoiced, fin.total_value, 12);
        lines.push(Line::from(vec![
            Span::styled("  Invoiced      ", theme::muted_style()),
            Span::styled(
                format!("{:<8}", format_currency(fin.total_invoiced)),
                theme::label_style(),
            ),
            Span::styled(invoiced_bar, theme::active_style()),
        ]));

        // Paid with progress bar
        let paid_bar = progress_bar(fin.total_paid, fin.total_value, 12);
        lines.push(Line::from(vec![
            Span::styled("  Paid          ", theme::muted_style()),
            Span::styled(
                format!("{:<8}", format_currency(fin.total_paid)),
                theme::success_style(),
            ),
            Span::styled(paid_bar, theme::success_style()),
        ]));

        // Outstanding
        lines.push(Line::from(vec![
            Span::styled("  Outstanding   ", theme::muted_style()),
            Span::styled(
                format_currency(fin.total_outstanding),
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

    lines.push(Line::from(""));

    // Team activity section
    let period = if app.activity_days == 1 {
        "Today"
    } else {
        "This Week"
    };
    lines.push(Line::from(Span::styled(
        format!("  TEAM ACTIVITY · {}", period),
        theme::title_style(),
    )));
    lines.push(Line::from(Span::styled(
        "  ─────────────────────────────",
        theme::muted_style(),
    )));

    if app.dashboard.activity.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No activity",
            theme::muted_style(),
        )));
    } else {
        // Show up to 15 entries to fit in the panel
        let max_entries = 15.min(area.height.saturating_sub(lines.len() as u16 + 1) as usize);
        for entry in app.dashboard.activity.iter().take(max_entries) {
            let source_tag = match entry.source.as_str() {
                "github" => "[gh]",
                "email" => "[em]",
                "calendar" => "[cal]",
                "meeting" => "[mtg]",
                _ => "[os]",
            };

            let source_style = match entry.source.as_str() {
                "github" => theme::active_style(),
                "email" | "calendar" | "meeting" => theme::warning_style(),
                _ => theme::muted_style(),
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", source_tag), source_style),
                Span::styled(
                    truncate(&entry.summary, 35),
                    theme::label_style(),
                ),
            ]));
        }
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

/// Format a dollar amount as "$XK" or "$X.XM".
fn format_currency(amount: f64) -> String {
    if amount >= 1_000_000.0 {
        format!("${:.1}M", amount / 1_000_000.0)
    } else if amount >= 1_000.0 {
        format!("${:.0}K", amount / 1_000.0)
    } else {
        format!("${:.0}", amount)
    }
}

/// Build a text progress bar: filled/total proportion mapped to `width` chars.
fn progress_bar(value: f64, max: f64, width: usize) -> String {
    if max <= 0.0 {
        return " ".repeat(width);
    }
    let ratio = (value / max).clamp(0.0, 1.0);
    let filled = (ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!(
        "{}{}",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty)
    )
}
