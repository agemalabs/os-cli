//! Terminal theme — inherits system/terminal colors (Stylix, Base16, etc).
//!
//! Uses ANSI color indices so the TUI follows whatever theme the user
//! has configured in their terminal. No hardcoded RGB values.

use ratatui::style::{Color, Modifier, Style};

// ---- Terminal-native colors ----
// These map to the user's configured terminal palette (Stylix, etc).

/// Primary accent — terminal blue.
pub const ACCENT: Color = Color::Blue;
/// Success — terminal green.
pub const SUCCESS: Color = Color::Green;
/// Warning — terminal yellow.
pub const WARNING: Color = Color::Yellow;
/// Danger — terminal red.
pub const DANGER: Color = Color::Red;
/// Muted text — terminal dark gray / bright black.
pub const MUTED: Color = Color::DarkGray;
/// Primary text — terminal default foreground.
pub const TEXT: Color = Color::Reset;
/// Bright text — terminal white.
pub const BRIGHT: Color = Color::White;

// ---- Styles ----

pub fn title_style() -> Style {
    Style::default().fg(BRIGHT).add_modifier(Modifier::BOLD)
}

pub fn active_style() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn muted_style() -> Style {
    Style::default().fg(MUTED)
}

pub fn warning_style() -> Style {
    Style::default().fg(WARNING).add_modifier(Modifier::BOLD)
}

pub fn success_style() -> Style {
    Style::default().fg(SUCCESS)
}

pub fn danger_style() -> Style {
    Style::default().fg(DANGER)
}

pub fn key_style() -> Style {
    Style::default().fg(ACCENT)
}

pub fn label_style() -> Style {
    Style::default().fg(TEXT)
}

// ---- Character language ----

pub const MARKER_OS: &str = "◈";
pub const MARKER_ACTIVE: &str = "◉";
pub const MARKER_INACTIVE: &str = "◎";
pub const MARKER_OPEN: &str = "○";
pub const MARKER_DONE: &str = "✓";
pub const MARKER_DECISION: &str = "▲";
pub const MARKER_OVERDUE: &str = "⚠";
pub const ARROW: &str = "→";
pub const BAR_FILL: &str = "█";
pub const BAR_EMPTY: &str = "░";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_are_distinct() {
        assert_ne!(title_style(), muted_style());
        assert_ne!(active_style(), danger_style());
    }

    #[test]
    fn uses_terminal_colors() {
        assert_eq!(ACCENT, Color::Blue);
        assert_eq!(MUTED, Color::DarkGray);
        assert_eq!(TEXT, Color::Reset);
    }
}
