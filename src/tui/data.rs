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
    #[serde(default)]
    pub team_count: usize,
}

/// Financial summary combining engagement values and Xero data.
#[derive(Debug, Clone, Default)]
pub struct Financials {
    pub total_value: f64,
    pub total_invoiced: f64,
    pub total_paid: f64,
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

/// Revenue chart data — multi-series: paid, invoiced, and projected per week.
#[derive(Debug, Clone, Default)]
pub struct RevenueChart {
    /// ISO date strings for each week's Monday.
    pub week_labels: Vec<String>,
    /// Paid revenue per week.
    pub paid: Vec<f64>,
    /// Invoiced (but unpaid) revenue per week.
    pub invoiced: Vec<f64>,
    /// Projected revenue per week (zero for historical, values for future).
    pub projected: Vec<f64>,
    /// Total revenue over the trailing 90 days (paid + invoiced).
    pub total_90d: f64,
    /// Average weekly revenue over the trailing 90 days.
    pub avg_weekly: f64,
}

/// Dashboard data loaded from the API.
#[derive(Debug, Clone, Default)]
pub struct DashboardData {
    pub projects: Vec<ProjectSummary>,
    pub pending_changes_count: usize,
    pub financials: Financials,
    pub activity: Vec<ActivityEntry>,
    pub activity_days: u32,
    pub revenue_chart: RevenueChart,
}

/// Fetch dashboard data from the API.
pub async fn fetch_dashboard(client: &ApiClient) -> anyhow::Result<DashboardData> {
    let projects_resp: serde_json::Value = client.get("/projects?limit=20").await?;
    let projects: Vec<ProjectSummary> = projects_resp["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    let mut p: ProjectSummary = serde_json::from_value(v.clone()).ok()?;
                    p.team_count = v["team"].as_array().map(|a| a.len()).unwrap_or(0);
                    Some(p)
                })
                .collect()
        })
        .unwrap_or_default();

    let changes_resp: serde_json::Value = client.get("/changes").await?;
    let pending_changes_count = changes_resp["data"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);

    // Sum engagement values from project data
    let engagement_total: f64 = projects_resp["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v["engagement_value"].as_f64())
                .sum()
        })
        .unwrap_or(0.0);

    // Fetch Xero financials (non-fatal if not connected)
    let financials = match client.get::<serde_json::Value>("/xero/financials").await {
        Ok(resp) => {
            let invoiced = resp["data"]["total_invoiced"].as_f64().unwrap_or(0.0);
            let outstanding = resp["data"]["total_outstanding"].as_f64().unwrap_or(0.0);
            let paid = invoiced - outstanding;
            Financials {
                total_value: engagement_total,
                total_invoiced: invoiced,
                total_paid: paid,
                total_outstanding: outstanding,
            }
        }
        Err(_) => Financials {
            total_value: engagement_total,
            ..Financials::default()
        },
    };

    // Fetch activity (non-fatal)
    let activity = fetch_activity(client, 1).await.unwrap_or_default();

    // Fetch revenue chart (non-fatal)
    let revenue_chart = fetch_revenue_chart(client).await.unwrap_or_default();

    Ok(DashboardData {
        projects,
        pending_changes_count,
        financials,
        activity,
        activity_days: 1,
        revenue_chart,
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

/// Fetch revenue chart data from the API.
pub async fn fetch_revenue_chart(client: &ApiClient) -> anyhow::Result<RevenueChart> {
    let resp: serde_json::Value = client.get("/revenue/chart").await?;
    let data = &resp["data"];

    let total_90d = data["total_90d"].as_f64().unwrap_or(0.0);
    let avg_weekly = data["avg_weekly"].as_f64().unwrap_or(0.0);

    let week_labels: Vec<String> = data["weeks"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let parse_series = |key: &str| -> Vec<f64> {
        data[key]
            .as_array()
            .map(|arr| arr.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect())
            .unwrap_or_default()
    };

    let paid = parse_series("paid");
    let invoiced = parse_series("invoiced");
    let projected = parse_series("projected");

    Ok(RevenueChart {
        week_labels,
        paid,
        invoiced,
        projected,
        total_90d,
        avg_weekly,
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
