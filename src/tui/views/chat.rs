//! Chat view — contextual AI Q&A within the TUI.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::tui::theme;

/// A single chat message.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Chat view state.
#[derive(Debug, Clone, Default)]
pub struct ChatState {
    /// Project slug for context (if navigated from project view).
    pub project_slug: Option<String>,
    /// Lead ID for context (if navigated from lead detail).
    pub lead_id: Option<String>,
    /// Chat history for this session.
    pub messages: Vec<ChatMessage>,
    /// Current input text.
    pub input: String,
    /// Whether we are waiting for a response.
    pub loading: bool,
    /// Scroll offset for messages.
    pub scroll: u16,
}

impl ChatState {
    /// Create a new chat state with optional project or lead context.
    pub fn new(project_slug: Option<String>, lead_id: Option<String>) -> Self {
        Self {
            project_slug,
            lead_id,
            messages: Vec::new(),
            input: String::new(),
            loading: false,
            scroll: 0,
        }
    }

    /// Get context label for display.
    pub fn context_label(&self) -> String {
        if let Some(slug) = &self.project_slug {
            slug.clone()
        } else if let Some(id) = &self.lead_id {
            format!("Lead {}", &id[..8.min(id.len())])
        } else {
            "Global".to_string()
        }
    }
}

/// Render the chat view.
pub fn render(frame: &mut Frame, area: Rect, state: &ChatState) {
    let now = chrono::Local::now();
    let time_str = now.format("%H:%M %a").to_string();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(6),   // messages
            Constraint::Length(3), // input
            Constraint::Length(3), // footer
        ])
        .split(area);

    // Header
    let context = state.context_label();
    let left = format!(" CHAT · {}", context);
    let right = format!("{} ", time_str);
    let pad_len = (area.width as usize).saturating_sub(left.len() + right.len() + 2);
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" CHAT ", theme::title_style()),
        Span::styled(format!("· {}", context), theme::muted_style()),
        Span::raw(" ".repeat(pad_len)),
        Span::styled(right, theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(header, chunks[0]);

    // Messages
    render_messages(frame, chunks[1], state);

    // Input
    let input = Paragraph::new(Line::from(vec![
        Span::styled("  ", theme::label_style()),
        Span::styled(theme::ARROW, theme::active_style()),
        Span::styled(" ", theme::label_style()),
        if state.loading {
            Span::styled("Thinking...", theme::muted_style())
        } else if state.input.is_empty() {
            Span::styled("Type your question...", theme::muted_style())
        } else {
            Span::styled(&state.input, theme::label_style())
        },
        if !state.loading && !state.input.is_empty() {
            Span::styled("█", theme::active_style())
        } else {
            Span::raw("")
        },
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::active_style()),
    );
    frame.render_widget(input, chunks[2]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Enter", theme::key_style()),
        Span::styled("·send  ", theme::muted_style()),
        Span::styled("Esc", theme::key_style()),
        Span::styled("·back", theme::muted_style()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::muted_style()),
    );
    frame.render_widget(footer, chunks[3]);
}

fn render_messages(frame: &mut Frame, area: Rect, state: &ChatState) {
    let mut lines: Vec<Line> = Vec::new();

    if state.messages.is_empty() && !state.loading {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Ask a question about your projects, leads, or anything in OS.",
            theme::muted_style(),
        )));
        lines.push(Line::from(Span::styled(
            "  Context will be pulled automatically based on where you came from.",
            theme::muted_style(),
        )));
    } else {
        for msg in &state.messages {
            lines.push(Line::from(""));
            match msg.role.as_str() {
                "user" => {
                    lines.push(Line::from(vec![
                        Span::styled("  You: ", theme::active_style()),
                        Span::styled(&msg.content, theme::label_style()),
                    ]));
                }
                "assistant" => {
                    lines.push(Line::from(Span::styled("  OS:", theme::success_style())));
                    // Wrap long responses line by line
                    for line in msg.content.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("      {}", line),
                            theme::label_style(),
                        )));
                    }
                }
                _ => {}
            }
        }

        if state.loading {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  OS: [answering...]",
                theme::muted_style(),
            )));
        }
    }

    let widget = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((state.scroll, 0));
    frame.render_widget(widget, area);
}
