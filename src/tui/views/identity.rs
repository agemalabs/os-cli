//! Identity view — onboarding flow + display + re-interview.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::tui::theme;

const QUESTIONS: &[(&str, &str)] = &[
    (
        "Who are you and what is your role?",
        "Your background, what you do at Agema Labs.",
    ),
    (
        "How do you approach problems?",
        "Your instincts, your process, what you do before others even know there's a problem.",
    ),
    (
        "How do you write? Paste an example if you want.",
        "Voice, tone, sentence style, formality level.",
    ),
    (
        "How do you communicate with your team?",
        "Direct or collaborative, how you give and receive feedback.",
    ),
    (
        "What are you focused on right now?",
        "What matters most this month, what you're working on.",
    ),
    (
        "What are you into outside of work?",
        "Hobbies, interests, what you geek out about.",
    ),
    (
        "Anything else the AI should know about working with you?",
        "Pet peeves, preferences, things that slow you down.",
    ),
];

/// Identity view state.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum IdentityMode {
    /// Viewing existing identity document.
    View { content: String, complete: bool },
    /// Onboarding: answering questions one at a time.
    Onboarding {
        step: usize,
        answers: Vec<String>,
        current_input: String,
    },
    /// Reviewing generated identity before saving.
    Review {
        generated: String,
        answers: Vec<String>,
    },
}

impl Default for IdentityMode {
    fn default() -> Self {
        Self::View {
            content: String::new(),
            complete: false,
        }
    }
}

/// Fetch identity state from API and determine mode.
pub async fn fetch(client: &crate::api_client::ApiClient) -> anyhow::Result<IdentityMode> {
    let resp: serde_json::Value = client.get("/identity").await?;

    if resp["data"].is_null() {
        return Ok(IdentityMode::Onboarding {
            step: 0,
            answers: vec![String::new(); 7],
            current_input: String::new(),
        });
    }

    let content = resp["data"]["content"].as_str().unwrap_or("").to_string();
    let complete = resp["data"]["identity_complete"].as_bool().unwrap_or(false);

    if !complete && content.is_empty() {
        Ok(IdentityMode::Onboarding {
            step: 0,
            answers: vec![String::new(); 7],
            current_input: String::new(),
        })
    } else {
        Ok(IdentityMode::View { content, complete })
    }
}

/// Generate identity from answers via API.
pub async fn generate(
    client: &crate::api_client::ApiClient,
    answers: &[String],
) -> anyhow::Result<String> {
    let resp: serde_json::Value = client
        .post(
            "/identity/generate",
            &serde_json::json!({ "answers": answers }),
        )
        .await?;

    Ok(resp["data"]["content"].as_str().unwrap_or("").to_string())
}

/// Handle key events for identity view. Returns true if consumed.
pub fn handle_key(mode: &mut IdentityMode, key: &KeyEvent) -> IdentityAction {
    match mode {
        IdentityMode::View { .. } => match key.code {
            KeyCode::Char('r') => IdentityAction::StartOnboarding,
            _ => IdentityAction::None,
        },
        IdentityMode::Onboarding {
            step,
            answers,
            current_input,
        } => match key.code {
            KeyCode::Enter => {
                answers[*step] = current_input.clone();
                if *step < 6 {
                    *step += 1;
                    *current_input = answers[*step].clone();
                    IdentityAction::None
                } else {
                    IdentityAction::Generate
                }
            }
            KeyCode::Backspace => {
                current_input.pop();
                IdentityAction::None
            }
            KeyCode::Left if *step > 0 => {
                answers[*step] = current_input.clone();
                *step -= 1;
                *current_input = answers[*step].clone();
                IdentityAction::None
            }
            KeyCode::Char(c) => {
                current_input.push(c);
                IdentityAction::None
            }
            _ => IdentityAction::None,
        },
        IdentityMode::Review { .. } => match key.code {
            KeyCode::Enter => IdentityAction::Save,
            KeyCode::Char('r') => IdentityAction::Regenerate,
            _ => IdentityAction::None,
        },
    }
}

/// Action returned from key handling.
pub enum IdentityAction {
    None,
    StartOnboarding,
    Generate,
    Regenerate,
    Save,
}

/// Render identity view based on current mode.
pub fn render(frame: &mut Frame, area: Rect, mode: &IdentityMode, user_name: &str) {
    match mode {
        IdentityMode::View { content, .. } => render_view(frame, area, content, user_name),
        IdentityMode::Onboarding {
            step,
            current_input,
            ..
        } => render_onboarding(frame, area, *step, current_input, user_name),
        IdentityMode::Review { generated, .. } => render_review(frame, area, generated),
    }
}

fn render_view(frame: &mut Frame, area: Rect, content: &str, user_name: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(" IDENTITY ", theme::title_style()),
        Span::styled(format!("· {}", user_name), theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(header, chunks[0]);

    let body = if content.is_empty() {
        Paragraph::new(Span::styled(
            "\n  No identity document yet. Press r to start the interview.",
            theme::muted_style(),
        ))
    } else {
        Paragraph::new(format!("\n{}", content)).wrap(Wrap { trim: false })
    };
    frame.render_widget(body, chunks[1]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" r", theme::key_style()),
        Span::styled("·re-interview  ", theme::muted_style()),
        Span::styled("b", theme::key_style()),
        Span::styled("·back", theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(footer, chunks[2]);
}

fn render_onboarding(frame: &mut Frame, area: Rect, step: usize, input: &str, user_name: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" IDENTITY ", theme::title_style()),
        Span::styled(
            format!("· {} · {} of 7", user_name, step + 1),
            theme::muted_style(),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(header, chunks[0]);

    // Question
    let (question, hint) = QUESTIONS[step];
    let q = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", question),
            theme::title_style(),
        )),
        Line::from(Span::styled(format!("  {}", hint), theme::muted_style())),
    ]);
    frame.render_widget(q, chunks[1]);

    // Input
    let input_widget = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(input, theme::label_style()),
            Span::styled("█", theme::active_style()),
        ]),
    ])
    .wrap(Wrap { trim: false });
    frame.render_widget(input_widget, chunks[2]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Enter", theme::key_style()),
        Span::styled("·next  ", theme::muted_style()),
        Span::styled("←", theme::key_style()),
        Span::styled("·back", theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(footer, chunks[3]);
}

fn render_review(frame: &mut Frame, area: Rect, content: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(" IDENTITY ", theme::title_style()),
        Span::styled("· Review", theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(header, chunks[0]);

    let body = Paragraph::new(format!("\n{}", content)).wrap(Wrap { trim: false });
    frame.render_widget(body, chunks[1]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Enter", theme::key_style()),
        Span::styled("·save  ", theme::muted_style()),
        Span::styled("r", theme::key_style()),
        Span::styled("·regenerate  ", theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(footer, chunks[3]);
}
