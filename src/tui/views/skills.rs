//! Skills view — browse and run skills.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::tui::theme;

/// Skills view state.
#[derive(Debug, Clone, Default)]
pub struct SkillsState {
    pub skills: Vec<SkillSummary>,
    pub selected: usize,
    pub loaded: bool,
    pub run_output: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SkillSummary {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub scope: String,
}

/// Fetch skills from the API.
pub async fn fetch(client: &crate::api_client::ApiClient) -> anyhow::Result<Vec<SkillSummary>> {
    let resp: serde_json::Value = client.get("/skills").await?;
    let skills = resp["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|s| SkillSummary {
                    slug: s["slug"].as_str().unwrap_or("").to_string(),
                    name: s["name"].as_str().unwrap_or("?").to_string(),
                    description: s["description"].as_str().unwrap_or("").to_string(),
                    scope: s["scope"].as_str().unwrap_or("personal").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(skills)
}

/// Run a skill with variables.
pub async fn run_skill(
    client: &crate::api_client::ApiClient,
    slug: &str,
    variables: &serde_json::Value,
) -> anyhow::Result<String> {
    let resp: serde_json::Value = client
        .post(
            &format!("/skills/{}/run", slug),
            &serde_json::json!({ "variables": variables }),
        )
        .await?;

    Ok(resp["data"]["output"]
        .as_str()
        .unwrap_or("No output")
        .to_string())
}

/// Render the skills view.
pub fn render(frame: &mut Frame, area: Rect, state: &SkillsState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" SKILLS", theme::title_style()),
        Span::styled(
            format!("  {} available", state.skills.len()),
            theme::muted_style(),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(header, chunks[0]);

    // Content
    if let Some(output) = &state.run_output {
        // Show skill output
        let body = Paragraph::new(format!("\n{}", output)).wrap(Wrap { trim: false });
        frame.render_widget(body, chunks[1]);
    } else {
        render_skill_list(frame, chunks[1], state);
    }

    // Footer
    let footer = if state.run_output.is_some() {
        Paragraph::new(Line::from(vec![
            Span::styled(" Esc", theme::key_style()),
            Span::styled("·back to list  ", theme::muted_style()),
            Span::styled("b", theme::key_style()),
            Span::styled("·back", theme::muted_style()),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled(" Enter", theme::key_style()),
            Span::styled("·run  ", theme::muted_style()),
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
}

fn render_skill_list(frame: &mut Frame, area: Rect, state: &SkillsState) {
    let mut lines = Vec::new();

    if !state.loaded {
        lines.push(Line::from(Span::styled(
            "  Loading...",
            theme::muted_style(),
        )));
    } else if state.skills.is_empty() {
        lines.push(Line::from(Span::styled(
            "\n  No skills configured.",
            theme::muted_style(),
        )));
    } else {
        for (i, skill) in state.skills.iter().enumerate() {
            let is_selected = i == state.selected;
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

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), style),
                Span::styled(&skill.name, style),
            ]));
            lines.push(Line::from(Span::styled(
                format!("    {}  · {}", skill.description, skill.scope),
                theme::muted_style(),
            )));
        }
    }

    let widget = Paragraph::new(lines);
    frame.render_widget(widget, area);
}
