//! Async data fetching for TUI views.

use crate::api_client::ApiClient;
use serde::Deserialize;

/// Project summary for dashboard display.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ProjectSummary {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub phase: String,
    pub is_internal: bool,
}

/// Dashboard data loaded from the API.
#[derive(Debug, Clone, Default)]
pub struct DashboardData {
    pub projects: Vec<ProjectSummary>,
    pub pending_changes_count: usize,
}

/// Fetch dashboard data from the API.
pub async fn fetch_dashboard(client: &ApiClient) -> anyhow::Result<DashboardData> {
    let projects_resp: serde_json::Value = client.get("/projects?limit=20").await?;
    let projects: Vec<ProjectSummary> = projects_resp["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    let changes_resp: serde_json::Value = client.get("/changes").await?;
    let pending_changes_count = changes_resp["data"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);

    Ok(DashboardData {
        projects,
        pending_changes_count,
    })
}

/// Fetch user info (name).
pub async fn fetch_user_name(client: &ApiClient) -> anyhow::Result<String> {
    let resp: serde_json::Value = client.get("/auth/me").await?;
    Ok(resp["data"]["name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string())
}
