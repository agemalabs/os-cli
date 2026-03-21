//! TUI application state machine.

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;

use crate::api_client::ApiClient;
use crate::tui::data::DashboardData;
use crate::tui::views::changes::ChangesState;
use crate::tui::views::identity::IdentityMode;
use crate::tui::views::lead::LeadDetail;
use crate::tui::views::pipeline::PipelineState;
use crate::tui::views::project::ProjectData;
use crate::tui::views::search::SearchState;
use crate::tui::views::skills::SkillsState;
use crate::tui::views::status::StatusData;

/// Which view is currently active.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum View {
    Dashboard,
    Status,
    Project { slug: String },
    Search,
    Changes,
    Pipeline,
    LeadDetail { id: String },
    Identity,
    Skills,
}

/// Top-level application state.
pub struct App {
    pub view: View,
    pub previous_view: Option<View>,
    pub running: bool,
    pub client: ApiClient,
    pub user_name: String,
    pub dashboard: DashboardData,
    pub project_data: Option<ProjectData>,
    pub status_data: Option<StatusData>,
    pub search: SearchState,
    pub changes: ChangesState,
    pub pipeline: PipelineState,
    pub lead_detail: Option<LeadDetail>,
    pub identity_mode: IdentityMode,
    pub skills: SkillsState,
    pub selected_index: usize,
    pub loading: bool,
    pub error: Option<String>,
}

impl App {
    /// Create a new app, optionally focused on a project.
    pub fn new(client: ApiClient, project: Option<String>) -> Self {
        let view = match project {
            Some(slug) => View::Project {
                slug: slug.trim_start_matches('@').to_string(),
            },
            None => View::Dashboard,
        };

        Self {
            view,
            previous_view: None,
            running: true,
            client,
            user_name: String::new(),
            dashboard: DashboardData::default(),
            project_data: None,
            status_data: None,
            search: SearchState::default(),
            changes: ChangesState::default(),
            pipeline: PipelineState::default(),
            lead_detail: None,
            identity_mode: IdentityMode::default(),
            skills: SkillsState::default(),
            selected_index: 0,
            loading: true,
            error: None,
        }
    }

    /// Navigate to a new view, saving the current one for back navigation.
    pub fn navigate(&mut self, view: View) {
        self.previous_view = Some(self.view.clone());
        self.view = view;
        self.selected_index = 0;
    }

    /// Go back to the previous view.
    pub fn go_back(&mut self) {
        if let Some(prev) = self.previous_view.take() {
            self.view = prev;
            self.selected_index = 0;
        }
    }

    /// Handle a global key event. Returns true if the key was consumed.
    pub fn handle_global_key(&mut self, key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') => {
                self.running = false;
                true
            }
            KeyCode::Char('b') => {
                if self.view != View::Dashboard {
                    self.go_back();
                    true
                } else {
                    false
                }
            }
            KeyCode::Char('i') => {
                // Identity data loaded in main loop after navigation
                self.navigate(View::Identity);
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        App::new(ApiClient::new("http://localhost", ""), None)
    }

    #[test]
    fn starts_at_dashboard() {
        let app = test_app();
        assert_eq!(app.view, View::Dashboard);
        assert!(app.running);
    }

    #[test]
    fn starts_at_project_when_given() {
        let app = App::new(
            ApiClient::new("http://localhost", ""),
            Some("@brownells".into()),
        );
        assert_eq!(
            app.view,
            View::Project {
                slug: "brownells".into()
            }
        );
    }

    #[test]
    fn navigate_and_back() {
        let mut app = test_app();
        app.navigate(View::Search);
        assert_eq!(app.view, View::Search);

        app.go_back();
        assert_eq!(app.view, View::Dashboard);
    }

    #[test]
    fn q_stops_app() {
        let mut app = test_app();
        let key = KeyEvent::new(KeyCode::Char('q'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_global_key(&key));
        assert!(!app.running);
    }

    #[test]
    fn b_does_not_go_back_from_dashboard() {
        let mut app = test_app();
        let key = KeyEvent::new(KeyCode::Char('b'), crossterm::event::KeyModifiers::NONE);
        assert!(!app.handle_global_key(&key));
        assert_eq!(app.view, View::Dashboard);
    }
}
