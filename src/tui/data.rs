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

/// Financial summary from Xero.
#[derive(Debug, Clone, Default)]
pub struct Financials {
    pub total_value: f64,
    pub total_invoiced: f64,
    pub total_outstanding: f64,
}

/// A single activity log entry.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ActivityEntry {
    pub user_name: String,
    pub summary: String,
    pub source: String,
    pub project_slug: Option<String>,
    pub created_at: String,
}

/// Dashboard data loaded from the API.
#[derive(Debug, Clone, Default)]
pub struct DashboardData {
    pub projects: Vec<ProjectSummary>,
    pub pending_changes_count: usize,
    pub financials: Financials,
    pub activity: Vec<ActivityEntry>,
    pub activity_days: u32,
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

    // Fetch Xero financials (non-fatal if not connected)
    let financials = match client.get::<serde_json::Value>("/xero/financials").await {
        Ok(resp) => Financials {
            total_value: resp["data"]["total_value"].as_f64().unwrap_or(0.0),
            total_invoiced: resp["data"]["total_invoiced"].as_f64().unwrap_or(0.0),
            total_outstanding: resp["data"]["total_outstanding"].as_f64().unwrap_or(0.0),
        },
        Err(_) => Financials::default(),
    };

    // Fetch activity (non-fatal)
    let activity = fetch_activity(client, 1).await.unwrap_or_default();

    Ok(DashboardData {
        projects,
        pending_changes_count,
        financials,
        activity,
        activity_days: 1,
    })
}

/// Fetch recent activity from the API.
pub async fn fetch_activity(client: &ApiClient, days: u32) -> anyhow::Result<Vec<ActivityEntry>> {
    let resp: serde_json::Value = client
        .get(&format!("/activity?days={}&limit=50", days))
        .await?;
    let entries = resp["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|v| ActivityEntry {
                    user_name: v["user_name"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string(),
                    summary: v["summary"].as_str().unwrap_or("").to_string(),
                    source: v["source"].as_str().unwrap_or("os").to_string(),
                    project_slug: v["project_slug"].as_str().map(|s| s.to_string()),
                    created_at: v["created_at"].as_str().unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(entries)
}

/// Fetch user info (name).
pub async fn fetch_user_name(client: &ApiClient) -> anyhow::Result<String> {
    let resp: serde_json::Value = client.get("/auth/me").await?;
    Ok(resp["data"]["name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string())
}
