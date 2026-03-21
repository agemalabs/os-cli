//! Agema Labs terminal theme — colors, styles, characters.

use ratatui::style::{Color, Modifier, Style};

// ---- Agema Labs palette ----

pub const NAVY: Color = Color::Rgb(15, 20, 25);
pub const BLUE: Color = Color::Rgb(37, 99, 235);
pub const TEXT: Color = Color::Rgb(55, 65, 81);
pub const MUTED: Color = Color::Rgb(107, 114, 128);
pub const WHITE: Color = Color::White;
pub const SUCCESS: Color = Color::Rgb(5, 150, 105);
pub const AMBER: Color = Color::Rgb(217, 119, 6);
pub const RED: Color = Color::Rgb(220, 38, 38);

// ---- Styles ----

pub fn title_style() -> Style {
    Style::default().fg(WHITE).add_modifier(Modifier::BOLD)
}

pub fn active_style() -> Style {
    Style::default().fg(BLUE).add_modifier(Modifier::BOLD)
}

pub fn muted_style() -> Style {
    Style::default().fg(MUTED)
}

pub fn warning_style() -> Style {
    Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
}

pub fn success_style() -> Style {
    Style::default().fg(SUCCESS)
}

pub fn danger_style() -> Style {
    Style::default().fg(RED)
}

pub fn key_style() -> Style {
    Style::default().fg(BLUE)
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
    fn colors_are_rgb() {
        assert!(matches!(BLUE, Color::Rgb(37, 99, 235)));
        assert!(matches!(NAVY, Color::Rgb(15, 20, 25)));
    }
}
