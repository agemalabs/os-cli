//! Terminal event handling — keyboard input and tick timer.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

/// Application event — either a key press or a periodic tick.
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
}

/// Poll for terminal events with a timeout.
/// Returns `Tick` if no event within the timeout period.
pub fn poll(timeout: Duration) -> Result<AppEvent> {
    if event::poll(timeout)? {
        if let Event::Key(key) = event::read()? {
            return Ok(AppEvent::Key(key));
        }
    }
    Ok(AppEvent::Tick)
}

/// Check if this key event is a quit signal.
pub fn is_quit(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q'))
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q_is_quit() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(is_quit(&key));
    }

    #[test]
    fn ctrl_c_is_quit() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(is_quit(&key));
    }

    #[test]
    fn other_keys_not_quit() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert!(!is_quit(&key));
    }
}
