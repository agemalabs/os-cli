//! Terminal UI — Ratatui + crossterm.

pub mod app;
pub mod data;
pub mod events;
#[allow(dead_code)]
pub mod theme;
pub mod views;

use anyhow::Result;
use crossterm::event::KeyCode;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
use std::io::stdout;
use std::time::Duration;

use crate::api_client::ApiClient;
use app::{App, View};

/// Launch the TUI.
pub async fn run(client: ApiClient, project: Option<String>) -> Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(client, project);

    // Fetch initial data
    load_initial_data(&mut app).await;

    let result = main_loop(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

/// Load initial data (user info + dashboard).
async fn load_initial_data(app: &mut App) {
    match data::fetch_user_name(&app.client).await {
        Ok(name) => app.user_name = name,
        Err(_) => app.user_name = "unknown".into(),
    }

    match data::fetch_dashboard(&app.client).await {
        Ok(d) => {
            app.dashboard = d;
            app.loading = false;
        }
        Err(e) => {
            app.error = Some(e.to_string());
            app.loading = false;
        }
    }

    // If started with a project, load its data
    if let View::Project { slug } = &app.view {
        if let Ok(pd) = views::project::fetch(&app.client, slug).await {
            app.project_data = Some(pd);
        }
    }
}

/// The main event/render loop.
async fn main_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    while app.running {
        terminal.draw(|frame| {
            let area = frame.size();
            match &app.view {
                View::Dashboard => views::dashboard::render(frame, area, app),
                View::Status => {
                    let default = views::status::StatusData::default();
                    let sd = app.status_data.as_ref().unwrap_or(&default);
                    views::status::render(frame, area, sd, &app.user_name);
                }
                View::Project { slug } => {
                    let default = views::project::ProjectData::default();
                    let pd = app.project_data.as_ref().unwrap_or(&default);
                    views::project::render(frame, area, app, slug, pd);
                }
                View::Search => {
                    views::search::render(frame, area, &app.search);
                }
                View::Changes => {
                    views::changes::render(frame, area, &app.changes);
                }
                View::Pipeline => {
                    views::pipeline::render(frame, area, &app.pipeline);
                }
                View::LeadDetail { .. } => {
                    let default = views::lead::LeadDetail::default();
                    let ld = app.lead_detail.as_ref().unwrap_or(&default);
                    views::lead::render(frame, area, ld);
                }
                View::Identity => {
                    views::identity::render(frame, area, &app.identity_mode, &app.user_name);
                }
                View::Skills => {
                    views::skills::render(frame, area, &app.skills);
                }
            }
        })?;

        match events::poll(Duration::from_millis(250))? {
            events::AppEvent::Key(key) => {
                // Search view captures all typing — handle before global keys
                if app.view == View::Search {
                    match key.code {
                        KeyCode::Esc => {
                            if app.search.query.is_empty() {
                                app.go_back();
                            } else {
                                app.search.query.clear();
                                app.search.results.clear();
                                app.search.searched = false;
                            }
                        }
                        KeyCode::Backspace => {
                            app.search.query.pop();
                        }
                        KeyCode::Enter => {
                            if !app.search.query.is_empty() {
                                app.search.searching = true;
                                match views::search::execute(&app.client, &app.search.query).await {
                                    Ok(results) => {
                                        app.search.results = results;
                                        app.search.selected = 0;
                                    }
                                    Err(_) => app.search.results.clear(),
                                }
                                app.search.searching = false;
                                app.search.searched = true;
                            }
                        }
                        KeyCode::Down => {
                            let max = app.search.results.len().saturating_sub(1);
                            if app.search.selected < max {
                                app.search.selected += 1;
                            }
                        }
                        KeyCode::Up => {
                            if app.search.selected > 0 {
                                app.search.selected -= 1;
                            }
                        }
                        KeyCode::Char(c) => {
                            if c == 'c'
                                && key
                                    .modifiers
                                    .contains(crossterm::event::KeyModifiers::CONTROL)
                            {
                                app.running = false;
                            } else {
                                app.search.query.push(c);
                            }
                        }
                        _ => {}
                    }
                    continue;
                }

                // Identity view captures typing during onboarding
                if app.view == View::Identity {
                    if key.code == KeyCode::Esc {
                        app.go_back();
                        continue;
                    }
                    if key.code == KeyCode::Char('c')
                        && key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL)
                    {
                        app.running = false;
                        continue;
                    }
                    let action = views::identity::handle_key(&mut app.identity_mode, &key);
                    match action {
                        views::identity::IdentityAction::StartOnboarding => {
                            app.identity_mode = views::identity::IdentityMode::Onboarding {
                                step: 0,
                                answers: vec![String::new(); 7],
                                current_input: String::new(),
                            };
                        }
                        views::identity::IdentityAction::Generate => {
                            if let views::identity::IdentityMode::Onboarding { answers, .. } =
                                &app.identity_mode
                            {
                                let answers = answers.clone();
                                if let Ok(content) =
                                    views::identity::generate(&app.client, &answers).await
                                {
                                    app.identity_mode = views::identity::IdentityMode::Review {
                                        generated: content,
                                        answers,
                                    };
                                }
                            }
                        }
                        views::identity::IdentityAction::Regenerate => {
                            if let views::identity::IdentityMode::Review { answers, .. } =
                                &app.identity_mode
                            {
                                let answers = answers.clone();
                                if let Ok(content) =
                                    views::identity::generate(&app.client, &answers).await
                                {
                                    app.identity_mode = views::identity::IdentityMode::Review {
                                        generated: content,
                                        answers,
                                    };
                                }
                            }
                        }
                        views::identity::IdentityAction::Save => {
                            if let views::identity::IdentityMode::Review { generated, .. } =
                                &app.identity_mode
                            {
                                let content = generated.clone();
                                // Save via PUT
                                let body = serde_json::json!({
                                    "content": content,
                                    "complete": true
                                });
                                let _ = app
                                    .client
                                    .post::<serde_json::Value>("/identity", &body)
                                    .await;
                                app.identity_mode = views::identity::IdentityMode::View {
                                    content,
                                    complete: true,
                                };
                            }
                        }
                        views::identity::IdentityAction::None => {}
                    }
                    continue;
                }

                if events::is_quit(&key) {
                    app.running = false;
                    continue;
                }

                if app.handle_global_key(&key) {
                    // Load identity data if we just navigated to identity view
                    if app.view == View::Identity {
                        if let Ok(mode) = views::identity::fetch(&app.client).await {
                            app.identity_mode = mode;
                        }
                    }
                    continue;
                }

                // View-specific keys
                if app.view == View::Changes {
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            let max = app.changes.changes.len().saturating_sub(1);
                            if app.changes.selected < max {
                                app.changes.selected += 1;
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if app.changes.selected > 0 {
                                app.changes.selected -= 1;
                            }
                        }
                        KeyCode::Char('a') => {
                            if let Some(change) = app.changes.changes.get(app.changes.selected) {
                                let id = change.id.clone();
                                if views::changes::approve(&app.client, &id).await.is_ok() {
                                    app.changes.changes.remove(app.changes.selected);
                                    if app.changes.selected > 0
                                        && app.changes.selected >= app.changes.changes.len()
                                    {
                                        app.changes.selected -= 1;
                                    }
                                }
                            }
                        }
                        KeyCode::Char('r') => {
                            if let Some(change) = app.changes.changes.get(app.changes.selected) {
                                let id = change.id.clone();
                                if views::changes::reject(&app.client, &id).await.is_ok() {
                                    app.changes.changes.remove(app.changes.selected);
                                    if app.changes.selected > 0
                                        && app.changes.selected >= app.changes.changes.len()
                                    {
                                        app.changes.selected -= 1;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                if app.view == View::Pipeline {
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            let max = app.pipeline.leads.len().saturating_sub(1);
                            if app.pipeline.selected < max {
                                app.pipeline.selected += 1;
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if app.pipeline.selected > 0 {
                                app.pipeline.selected -= 1;
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(lead) = app.pipeline.leads.get(app.pipeline.selected) {
                                let id = lead.id.clone();
                                match views::lead::fetch(&app.client, &id).await {
                                    Ok(ld) => app.lead_detail = Some(ld),
                                    Err(_) => {
                                        app.lead_detail = Some(views::lead::LeadDetail::default())
                                    }
                                }
                                app.navigate(View::LeadDetail { id });
                            }
                        }
                        _ => {}
                    }
                }

                if app.view == View::Dashboard {
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            let max = app.dashboard.projects.len().saturating_sub(1);
                            if app.selected_index < max {
                                app.selected_index += 1;
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if app.selected_index > 0 {
                                app.selected_index -= 1;
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(project) = app.dashboard.projects.get(app.selected_index) {
                                let slug = project.slug.clone();
                                // Fetch project data before navigating
                                match views::project::fetch(&app.client, &slug).await {
                                    Ok(pd) => app.project_data = Some(pd),
                                    Err(_) => {
                                        app.project_data =
                                            Some(views::project::ProjectData::default())
                                    }
                                }
                                app.navigate(View::Project { slug });
                            }
                        }
                        KeyCode::Char('l') => {
                            match views::status::fetch(&app.client).await {
                                Ok(sd) => app.status_data = Some(sd),
                                Err(_) => {
                                    app.status_data = Some(views::status::StatusData::default())
                                }
                            }
                            app.navigate(View::Status);
                        }
                        KeyCode::Char('s') => app.navigate(View::Search),
                        KeyCode::Char('c') => {
                            match views::changes::fetch(&app.client).await {
                                Ok(c) => {
                                    app.changes = views::changes::ChangesState {
                                        changes: c,
                                        selected: 0,
                                        loaded: true,
                                    };
                                }
                                Err(_) => {
                                    app.changes.loaded = true;
                                    app.changes.changes.clear();
                                }
                            }
                            app.navigate(View::Changes);
                        }
                        KeyCode::Char('/') => {
                            match views::pipeline::fetch(&app.client).await {
                                Ok(leads) => {
                                    app.pipeline = views::pipeline::PipelineState {
                                        leads,
                                        selected: 0,
                                        loaded: true,
                                    };
                                }
                                Err(_) => {
                                    app.pipeline.loaded = true;
                                    app.pipeline.leads.clear();
                                }
                            }
                            app.navigate(View::Pipeline);
                        }
                        KeyCode::Char('K') => {
                            match views::skills::fetch(&app.client).await {
                                Ok(s) => {
                                    app.skills = views::skills::SkillsState {
                                        skills: s,
                                        selected: 0,
                                        loaded: true,
                                        run_output: None,
                                    };
                                }
                                Err(_) => {
                                    app.skills.loaded = true;
                                    app.skills.skills.clear();
                                }
                            }
                            app.navigate(View::Skills);
                        }
                        _ => {}
                    }
                }

                // Skills view
                if app.view == View::Skills {
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            let max = app.skills.skills.len().saturating_sub(1);
                            if app.skills.selected < max {
                                app.skills.selected += 1;
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if app.skills.selected > 0 {
                                app.skills.selected -= 1;
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(skill) = app.skills.skills.get(app.skills.selected) {
                                let slug = skill.slug.clone();
                                match views::skills::run_skill(
                                    &app.client,
                                    &slug,
                                    &serde_json::json!({}),
                                )
                                .await
                                {
                                    Ok(output) => app.skills.run_output = Some(output),
                                    Err(e) => app.skills.run_output = Some(format!("Error: {}", e)),
                                }
                            }
                        }
                        KeyCode::Esc => {
                            app.skills.run_output = None;
                        }
                        _ => {}
                    }
                }
            }
            events::AppEvent::Tick => {}
        }
    }

    Ok(())
}
