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
                    let input_active = matches!(
                        &app.input_mode,
                        Some(app::InputMode::ResolveDecision { .. })
                    );
                    views::status::render(
                        frame,
                        area,
                        sd,
                        &app.user_name,
                        input_active,
                        &app.input_buffer,
                    );
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
                View::Chat => {
                    views::chat::render(frame, area, &app.chat);
                }
                View::Clients => {
                    views::clients::render_list(frame, area, &app.clients, &app.user_name);
                }
                View::ClientDetail { .. } => {
                    let default = views::clients::ClientDetailData::default();
                    let detail = app.clients.detail.as_ref().unwrap_or(&default);
                    views::clients::render_detail(frame, area, detail, &app.user_name);
                }
            }
        })?;

        match events::poll(Duration::from_millis(250))? {
            events::AppEvent::Key(key) => {
                // Input mode captures all typing
                if app.input_mode.is_some() {
                    handle_input_mode(app, &key).await;
                    continue;
                }

                // Chat view captures typing
                if app.view == View::Chat {
                    handle_chat_keys(app, &key).await;
                    continue;
                }

                // Search view captures all typing
                if app.view == View::Search {
                    handle_search_keys(app, &key).await;
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
                    handle_identity_keys(app, &key).await;
                    continue;
                }

                // Pipeline filter mode captures typing
                if app.view == View::Pipeline && app.pipeline.filtering {
                    handle_pipeline_filter(app, &key).await;
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
                match &app.view {
                    View::Changes => handle_changes_keys(app, &key).await,
                    View::Pipeline => handle_pipeline_keys(app, &key).await,
                    View::LeadDetail { .. } => handle_lead_detail_keys(app, &key).await,
                    View::Project { .. } => handle_project_keys(app, &key),
                    View::Dashboard => handle_dashboard_keys(app, &key).await,
                    View::Skills => handle_skills_keys(app, &key).await,
                    View::Status => handle_status_keys(app, &key).await,
                    View::Clients => handle_clients_keys(app, &key).await,
                    View::ClientDetail { .. } => handle_client_detail_keys(app, &key).await,
                    _ => {}
                }
            }
            events::AppEvent::Tick => {}
        }
    }

    Ok(())
}

/// Handle input mode key events.
async fn handle_input_mode(app: &mut App, key: &crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = None;
            app.input_buffer.clear();
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Enter => {
            let buffer = app.input_buffer.clone();
            let mode = app.input_mode.take();
            app.input_buffer.clear();

            if !buffer.is_empty() {
                match mode {
                    Some(app::InputMode::PushFile { project_slug }) => {
                        if let Ok(_msg) =
                            views::project::push_file(&app.client, &project_slug, &buffer).await
                        {
                            if let Ok(pd) =
                                views::project::fetch(&app.client, &project_slug).await
                            {
                                app.project_data = Some(pd);
                            }
                        }
                    }
                    Some(app::InputMode::LinkRepo { project_slug }) => {
                        let _ = app
                            .client
                            .link_repo(&project_slug, &buffer, None)
                            .await;
                        if let Ok(pd) =
                            views::project::fetch(&app.client, &project_slug).await
                        {
                            app.project_data = Some(pd);
                        }
                    }
                    Some(app::InputMode::NewTask { project_slug }) => {
                        let body = serde_json::json!({ "title": buffer });
                        let _ = app
                            .client
                            .post::<serde_json::Value>(
                                &format!("/projects/{}/tasks", project_slug),
                                &body,
                            )
                            .await;
                        if let Ok(pd) =
                            views::project::fetch(&app.client, &project_slug).await
                        {
                            app.project_data = Some(pd);
                        }
                    }
                    Some(app::InputMode::NewDecision { project_slug }) => {
                        let _ = app
                            .client
                            .create_decision(&project_slug, &buffer, None)
                            .await;
                        if let Ok(pd) =
                            views::project::fetch(&app.client, &project_slug).await
                        {
                            app.project_data = Some(pd);
                        }
                    }
                    Some(app::InputMode::NewLead) => {
                        let body = serde_json::json!({ "company_name": buffer });
                        let _ = app
                            .client
                            .post::<serde_json::Value>("/leads", &body)
                            .await;
                        // Reload pipeline
                        if let Ok(leads) = views::pipeline::fetch(&app.client).await {
                            app.pipeline.leads = leads;
                        }
                    }
                    Some(app::InputMode::NewProject) => {
                        // Format: "name:slug" or just "name" (slug auto-generated)
                        let parts: Vec<&str> = buffer.splitn(2, ':').collect();
                        let name = parts[0].trim();
                        let slug = if parts.len() > 1 {
                            parts[1].trim().to_string()
                        } else {
                            name.to_lowercase()
                                .replace(|c: char| !c.is_alphanumeric() && c != '-', "-")
                        };
                        let _ = app.client.create_project(name, &slug, None).await;
                        // Reload dashboard
                        if let Ok(d) = data::fetch_dashboard(&app.client).await {
                            app.dashboard = d;
                        }
                    }
                    Some(app::InputMode::AddLeadNote { lead_id }) => {
                        let _ = app.client.add_lead_note(&lead_id, &buffer).await;
                        // Reload lead detail
                        if let Ok(ld) = views::lead::fetch(&app.client, &lead_id).await {
                            app.lead_detail = Some(ld);
                        }
                    }
                    Some(app::InputMode::AddLeadContact { lead_id }) => {
                        // Format: "name:email"
                        let parts: Vec<&str> = buffer.splitn(2, ':').collect();
                        let name = parts[0].trim();
                        let email = parts.get(1).map(|s| s.trim());
                        let mut contact = serde_json::json!({ "name": name });
                        if let Some(e) = email {
                            contact["email"] = serde_json::json!(e);
                        }
                        let _ = app
                            .client
                            .add_lead_contact(&lead_id, &contact)
                            .await;
                        if let Ok(ld) = views::lead::fetch(&app.client, &lead_id).await {
                            app.lead_detail = Some(ld);
                        }
                    }
                    Some(app::InputMode::ResolveDecision {
                        project_slug,
                        decision_id,
                    }) => {
                        let _ = app
                            .client
                            .resolve_decision(&project_slug, &decision_id, &buffer)
                            .await;
                        // Reload status
                        if let Ok(sd) = views::status::fetch(&app.client).await {
                            app.status_data = Some(sd);
                        }
                    }
                    Some(app::InputMode::ChatInput { project_slug, lead_id }) => {
                        // Start chat view with the question
                        app.chat = views::chat::ChatState::new(project_slug, lead_id);
                        app.chat.messages.push(views::chat::ChatMessage {
                            role: "user".to_string(),
                            content: buffer.clone(),
                        });
                        app.chat.loading = true;
                        app.navigate(View::Chat);

                        // Send to API
                        match app
                            .client
                            .chat(
                                &buffer,
                                app.chat.project_slug.as_deref(),
                                app.chat.lead_id.as_deref(),
                            )
                            .await
                        {
                            Ok(resp) => {
                                let answer = resp["data"]["answer"]
                                    .as_str()
                                    .unwrap_or("No response")
                                    .to_string();
                                app.chat.messages.push(views::chat::ChatMessage {
                                    role: "assistant".to_string(),
                                    content: answer,
                                });
                            }
                            Err(e) => {
                                app.chat.messages.push(views::chat::ChatMessage {
                                    role: "assistant".to_string(),
                                    content: format!("Error: {}", e),
                                });
                            }
                        }
                        app.chat.loading = false;
                    }
                    None => {}
                }
            }
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
}

/// Handle chat view key events.
async fn handle_chat_keys(app: &mut App, key: &crossterm::event::KeyEvent) {
    if app.chat.loading {
        // Only allow Esc while loading
        if key.code == KeyCode::Esc {
            app.go_back();
        }
        return;
    }

    match key.code {
        KeyCode::Esc => {
            app.go_back();
        }
        KeyCode::Backspace => {
            app.chat.input.pop();
        }
        KeyCode::Enter => {
            if !app.chat.input.is_empty() {
                let question = app.chat.input.clone();
                app.chat.input.clear();

                app.chat.messages.push(views::chat::ChatMessage {
                    role: "user".to_string(),
                    content: question.clone(),
                });
                app.chat.loading = true;

                match app
                    .client
                    .chat(
                        &question,
                        app.chat.project_slug.as_deref(),
                        app.chat.lead_id.as_deref(),
                    )
                    .await
                {
                    Ok(resp) => {
                        let answer = resp["data"]["answer"]
                            .as_str()
                            .unwrap_or("No response")
                            .to_string();
                        app.chat.messages.push(views::chat::ChatMessage {
                            role: "assistant".to_string(),
                            content: answer,
                        });
                    }
                    Err(e) => {
                        app.chat.messages.push(views::chat::ChatMessage {
                            role: "assistant".to_string(),
                            content: format!("Error: {}", e),
                        });
                    }
                }
                app.chat.loading = false;
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
                app.chat.input.push(c);
            }
        }
        _ => {}
    }
}

/// Handle search view key events.
async fn handle_search_keys(app: &mut App, key: &crossterm::event::KeyEvent) {
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
}

/// Handle identity view key events.
async fn handle_identity_keys(app: &mut App, key: &crossterm::event::KeyEvent) {
    let action = views::identity::handle_key(&mut app.identity_mode, key);
    match action {
        views::identity::IdentityAction::StartOnboarding => {
            app.identity_mode = views::identity::IdentityMode::Onboarding {
                step: 0,
                answers: vec![String::new(); 7],
                current_input: String::new(),
            };
        }
        views::identity::IdentityAction::Generate => {
            if let views::identity::IdentityMode::Onboarding { answers, .. } = &app.identity_mode {
                let answers = answers.clone();
                if let Ok(content) = views::identity::generate(&app.client, &answers).await {
                    app.identity_mode = views::identity::IdentityMode::Review {
                        generated: content,
                        answers,
                    };
                }
            }
        }
        views::identity::IdentityAction::Regenerate => {
            if let views::identity::IdentityMode::Review { answers, .. } = &app.identity_mode {
                let answers = answers.clone();
                if let Ok(content) = views::identity::generate(&app.client, &answers).await {
                    app.identity_mode = views::identity::IdentityMode::Review {
                        generated: content,
                        answers,
                    };
                }
            }
        }
        views::identity::IdentityAction::Save => {
            if let views::identity::IdentityMode::Review { generated, .. } = &app.identity_mode {
                let content = generated.clone();
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
}

/// Handle changes view key events.
async fn handle_changes_keys(app: &mut App, key: &crossterm::event::KeyEvent) {
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

/// Handle pipeline view key events.
async fn handle_pipeline_keys(app: &mut App, key: &crossterm::event::KeyEvent) {
    // Handle delete confirmation
    if app.pipeline.confirm_delete {
        match key.code {
            KeyCode::Char('y') => {
                let filtered = app.pipeline.filtered_leads();
                if let Some(lead) = filtered.get(app.pipeline.selected) {
                    let id = lead.id.clone();
                    let _ = app.client.delete_lead(&id).await;
                    // Remove from list
                    app.pipeline.leads.retain(|l| l.id != id);
                    if app.pipeline.selected > 0
                        && app.pipeline.selected >= app.pipeline.filtered_leads().len()
                    {
                        app.pipeline.selected = app.pipeline.selected.saturating_sub(1);
                    }
                }
                app.pipeline.confirm_delete = false;
            }
            _ => {
                app.pipeline.confirm_delete = false;
            }
        }
        return;
    }

    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            let max = app.pipeline.filtered_leads().len().saturating_sub(1);
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
            let filtered = app.pipeline.filtered_leads();
            if let Some(lead) = filtered.get(app.pipeline.selected) {
                let id = lead.id.clone();
                match views::lead::fetch(&app.client, &id).await {
                    Ok(ld) => app.lead_detail = Some(ld),
                    Err(_) => app.lead_detail = Some(views::lead::LeadDetail::default()),
                }
                app.navigate(View::LeadDetail { id });
            }
        }
        KeyCode::Char('n') => {
            app.input_mode = Some(app::InputMode::NewLead);
            app.input_buffer.clear();
        }
        KeyCode::Char('d') => {
            if !app.pipeline.filtered_leads().is_empty() {
                app.pipeline.confirm_delete = true;
            }
        }
        KeyCode::Char('/') => {
            app.pipeline.filtering = true;
            app.pipeline.filter = Some(String::new());
            app.pipeline.selected = 0;
        }
        _ => {}
    }
}

/// Handle pipeline filter input.
async fn handle_pipeline_filter(app: &mut App, key: &crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.pipeline.filtering = false;
            app.pipeline.filter = None;
            app.pipeline.selected = 0;
        }
        KeyCode::Backspace => {
            if let Some(f) = &mut app.pipeline.filter {
                f.pop();
            }
        }
        KeyCode::Enter => {
            app.pipeline.filtering = false;
            // Keep the filter active but stop capturing input
        }
        KeyCode::Char(c) => {
            if let Some(f) = &mut app.pipeline.filter {
                f.push(c);
            }
            app.pipeline.selected = 0;
        }
        _ => {}
    }
}

/// Handle lead detail view key events.
async fn handle_lead_detail_keys(app: &mut App, key: &crossterm::event::KeyEvent) {
    // Handle delete confirmation
    if let Some(ld) = &app.lead_detail {
        if ld.confirm_delete {
            match key.code {
                KeyCode::Char('y') => {
                    let id = ld.id.clone();
                    let _ = app.client.delete_lead(&id).await;
                    if let Some(ld) = &mut app.lead_detail {
                        ld.confirm_delete = false;
                    }
                    app.go_back();
                    // Reload pipeline
                    if let Ok(leads) = views::pipeline::fetch(&app.client).await {
                        app.pipeline.leads = leads;
                    }
                    return;
                }
                _ => {
                    if let Some(ld) = &mut app.lead_detail {
                        ld.confirm_delete = false;
                    }
                    return;
                }
            }
        }

        // Handle stage picker
        if ld.stage_picker.is_some() {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if let Some(ld) = &mut app.lead_detail {
                        if let Some(picker) = &mut ld.stage_picker {
                            if picker.selected < picker.stages.len() - 1 {
                                picker.selected += 1;
                            }
                        }
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if let Some(ld) = &mut app.lead_detail {
                        if let Some(picker) = &mut ld.stage_picker {
                            if picker.selected > 0 {
                                picker.selected -= 1;
                            }
                        }
                    }
                }
                KeyCode::Enter => {
                    let (id, stage) = {
                        let ld = app.lead_detail.as_ref().unwrap();
                        let stage = ld.stage_picker.as_ref().unwrap().current().to_string();
                        (ld.id.clone(), stage)
                    };
                    let updates = serde_json::json!({ "stage": stage });
                    let _ = app.client.update_lead(&id, &updates).await;
                    // Reload
                    if let Ok(ld) = views::lead::fetch(&app.client, &id).await {
                        app.lead_detail = Some(ld);
                    }
                }
                KeyCode::Esc => {
                    if let Some(ld) = &mut app.lead_detail {
                        ld.stage_picker = None;
                    }
                }
                _ => {}
            }
            return;
        }
    }

    match key.code {
        KeyCode::Char('n') => {
            if let Some(ld) = &app.lead_detail {
                let id = ld.id.clone();
                app.input_mode = Some(app::InputMode::AddLeadNote { lead_id: id });
                app.input_buffer.clear();
            }
        }
        KeyCode::Char('s') => {
            if let Some(ld) = &mut app.lead_detail {
                let stage = ld.stage.clone();
                ld.stage_picker = Some(views::lead::StagePicker::new(&stage));
            }
        }
        KeyCode::Char('a') => {
            if let Some(ld) = &app.lead_detail {
                let id = ld.id.clone();
                app.input_mode = Some(app::InputMode::AddLeadContact { lead_id: id });
                app.input_buffer.clear();
            }
        }
        KeyCode::Char('d') => {
            if let Some(ld) = &mut app.lead_detail {
                ld.confirm_delete = true;
            }
        }
        KeyCode::Char('c') => {
            if let Some(ld) = &app.lead_detail {
                let lead_id = ld.id.clone();
                app.chat = views::chat::ChatState::new(None, Some(lead_id));
                app.navigate(View::Chat);
            }
        }
        _ => {}
    }
}

/// Handle project view key events.
fn handle_project_keys(app: &mut App, key: &crossterm::event::KeyEvent) {
    if let View::Project { slug } = &app.view {
        let slug = slug.clone();
        match key.code {
            KeyCode::Char('p') => {
                app.input_mode = Some(app::InputMode::PushFile {
                    project_slug: slug,
                });
                app.input_buffer.clear();
            }
            KeyCode::Char('t') => {
                app.input_mode = Some(app::InputMode::NewTask {
                    project_slug: slug,
                });
                app.input_buffer.clear();
            }
            KeyCode::Char('d') => {
                app.input_mode = Some(app::InputMode::NewDecision {
                    project_slug: slug,
                });
                app.input_buffer.clear();
            }
            KeyCode::Char('r') => {
                app.input_mode = Some(app::InputMode::LinkRepo {
                    project_slug: slug,
                });
                app.input_buffer.clear();
            }
            KeyCode::Char('c') => {
                app.chat = views::chat::ChatState::new(Some(slug), None);
                app.navigate(View::Chat);
            }
            _ => {}
        }
    }
}

/// Handle dashboard view key events.
async fn handle_dashboard_keys(app: &mut App, key: &crossterm::event::KeyEvent) {
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
                match views::project::fetch(&app.client, &slug).await {
                    Ok(pd) => app.project_data = Some(pd),
                    Err(_) => app.project_data = Some(views::project::ProjectData::default()),
                }
                app.navigate(View::Project { slug });
            }
        }
        KeyCode::Char('l') => {
            match views::status::fetch(&app.client).await {
                Ok(sd) => app.status_data = Some(sd),
                Err(_) => app.status_data = Some(views::status::StatusData::default()),
            }
            app.navigate(View::Status);
        }
        KeyCode::Char('s') => app.navigate(View::Search),
        KeyCode::Char('c') => {
            app.chat = views::chat::ChatState::new(None, None);
            app.navigate(View::Chat);
        }
        KeyCode::Char('n') => {
            app.input_mode = Some(app::InputMode::NewProject);
            app.input_buffer.clear();
        }
        KeyCode::Char('x') => {
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
                        filter: None,
                        filtering: false,
                        confirm_delete: false,
                    };
                }
                Err(_) => {
                    app.pipeline.loaded = true;
                    app.pipeline.leads.clear();
                }
            }
            app.navigate(View::Pipeline);
        }
        KeyCode::Char('w') => {
            // Toggle activity between 1 day and 7 days
            app.activity_days = if app.activity_days == 1 { 7 } else { 1 };
            match data::fetch_activity(&app.client, app.activity_days).await {
                Ok(activity) => {
                    app.dashboard.activity = activity;
                    app.dashboard.activity_days = app.activity_days;
                }
                Err(_) => {
                    app.dashboard.activity.clear();
                }
            }
        }
        KeyCode::Char('C') => {
            match views::clients::fetch_clients(&app.client).await {
                Ok(c) => {
                    app.clients = views::clients::ClientsState {
                        clients: c,
                        selected: 0,
                        loaded: true,
                        detail: None,
                    };
                }
                Err(_) => {
                    app.clients.loaded = true;
                    app.clients.clients.clear();
                }
            }
            app.navigate(View::Clients);
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

/// Handle status view key events.
async fn handle_status_keys(app: &mut App, key: &crossterm::event::KeyEvent) {
    let sd = match &mut app.status_data {
        Some(sd) => sd,
        None => return,
    };

    match key.code {
        // Navigation within section
        KeyCode::Char('j') | KeyCode::Down => {
            let max = match sd.selected_section {
                views::status::StatusSection::Tasks => {
                    (sd.overdue.len() + sd.tasks.len()).saturating_sub(1)
                }
                views::status::StatusSection::Decisions => {
                    sd.decisions.len().saturating_sub(1)
                }
            };
            if sd.selected_index < max {
                sd.selected_index += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if sd.selected_index > 0 {
                sd.selected_index -= 1;
            }
        }
        // Tab to switch between Tasks and Decisions sections
        KeyCode::Tab => {
            sd.selected_section = match sd.selected_section {
                views::status::StatusSection::Tasks => views::status::StatusSection::Decisions,
                views::status::StatusSection::Decisions => views::status::StatusSection::Tasks,
            };
            sd.selected_index = 0;
        }
        // Enter on a task cycles status: open -> in_progress -> done
        KeyCode::Enter => {
            if sd.selected_section == views::status::StatusSection::Tasks {
                let all_tasks: Vec<views::status::StatusTask> =
                    sd.overdue.iter().chain(sd.tasks.iter()).cloned().collect();
                if let Some(task) = all_tasks.get(sd.selected_index) {
                    let new_status = match task.status.as_str() {
                        "open" => "in_progress",
                        "in_progress" => "done",
                        _ => "open",
                    };
                    let task_id = task.id.clone();
                    let project_slug = task.project_slug.clone();
                    let updates = serde_json::json!({ "status": new_status });
                    let _ = app
                        .client
                        .update_task(&project_slug, &task_id, &updates)
                        .await;
                    // Reload status
                    if let Ok(mut new_sd) = views::status::fetch(&app.client).await {
                        // Preserve selection state
                        let prev_section = sd.selected_section.clone();
                        let prev_index = sd.selected_index;
                        new_sd.selected_section = prev_section;
                        new_sd.selected_index = prev_index;
                        // Clamp index
                        let max = match new_sd.selected_section {
                            views::status::StatusSection::Tasks => {
                                (new_sd.overdue.len() + new_sd.tasks.len()).saturating_sub(1)
                            }
                            views::status::StatusSection::Decisions => {
                                new_sd.decisions.len().saturating_sub(1)
                            }
                        };
                        if new_sd.selected_index > max {
                            new_sd.selected_index = max;
                        }
                        app.status_data = Some(new_sd);
                    }
                }
            }
        }
        // 'r' to resolve a decision
        KeyCode::Char('r') => {
            if sd.selected_section == views::status::StatusSection::Decisions {
                if let Some(decision) = sd.decisions.get(sd.selected_index) {
                    let project_slug = decision.project.clone();
                    let decision_id = decision.id.clone();
                    app.input_mode = Some(app::InputMode::ResolveDecision {
                        project_slug,
                        decision_id,
                    });
                    app.input_buffer.clear();
                }
            }
        }
        // 'c' for chat from status
        KeyCode::Char('c') => {
            app.chat = views::chat::ChatState::new(None, None);
            app.navigate(View::Chat);
        }
        _ => {}
    }
}

/// Handle clients list view key events.
async fn handle_clients_keys(app: &mut App, key: &crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            let max = app.clients.clients.len().saturating_sub(1);
            if app.clients.selected < max {
                app.clients.selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.clients.selected > 0 {
                app.clients.selected -= 1;
            }
        }
        KeyCode::Enter => {
            if let Some(client) = app.clients.clients.get(app.clients.selected) {
                let slug = client.slug.clone();
                match views::clients::fetch_client_detail(&app.client, &slug).await {
                    Ok(detail) => app.clients.detail = Some(detail),
                    Err(_) => {
                        app.clients.detail = Some(views::clients::ClientDetailData::default())
                    }
                }
                app.navigate(View::ClientDetail { slug });
            }
        }
        _ => {}
    }
}

/// Handle client detail view key events.
async fn handle_client_detail_keys(app: &mut App, key: &crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(detail) = &mut app.clients.detail {
                let max = detail.projects.len().saturating_sub(1);
                if detail.selected < max {
                    detail.selected += 1;
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Some(detail) = &mut app.clients.detail {
                if detail.selected > 0 {
                    detail.selected -= 1;
                }
            }
        }
        KeyCode::Enter => {
            if let Some(detail) = &app.clients.detail {
                if let Some(project) = detail.projects.get(detail.selected) {
                    let slug = project.slug.clone();
                    match views::project::fetch(&app.client, &slug).await {
                        Ok(pd) => app.project_data = Some(pd),
                        Err(_) => {
                            app.project_data = Some(views::project::ProjectData::default())
                        }
                    }
                    app.navigate(View::Project { slug });
                }
            }
        }
        _ => {}
    }
}

/// Handle skills view key events.
async fn handle_skills_keys(app: &mut App, key: &crossterm::event::KeyEvent) {
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
                match views::skills::run_skill(&app.client, &slug, &serde_json::json!({})).await {
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
