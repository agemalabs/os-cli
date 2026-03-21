//! `os task` and `os tasks` — create and list tasks.

use crate::api_client::ApiClient;
use anyhow::Result;

/// Create a task in a project.
pub async fn create(
    client: &ApiClient,
    title: &str,
    project: &str,
    assign: Option<&str>,
    due: Option<&str>,
) -> Result<()> {
    let slug = project.trim_start_matches('@');

    let mut body = serde_json::json!({
        "title": title,
    });

    if let Some(assign) = assign {
        body["assign"] = serde_json::json!(assign);
    }
    if let Some(due) = due {
        body["due_date"] = serde_json::json!(due);
    }

    let resp: serde_json::Value = client
        .post(&format!("/projects/{}/tasks", slug), &body)
        .await?;

    let id = resp["data"]["id"].as_str().unwrap_or("?");
    println!("Created task: {} ({})", title, id);

    Ok(())
}

/// List tasks for a project.
pub async fn list(client: &ApiClient, project: &str) -> Result<()> {
    let slug = project.trim_start_matches('@');

    let resp: serde_json::Value = client.get(&format!("/projects/{}/tasks", slug)).await?;
    let tasks = resp["data"].as_array();

    match tasks {
        Some(tasks) if !tasks.is_empty() => {
            println!("\nTasks for @{}:\n", slug);
            for task in tasks {
                let title = task["title"].as_str().unwrap_or("?");
                let status = task["status"].as_str().unwrap_or("open");
                let due = task["due_date"].as_str().unwrap_or("");

                let marker = match status {
                    "done" => "✓",
                    "in_progress" => "▲",
                    _ => "○",
                };

                if due.is_empty() {
                    println!("  {} {}", marker, title);
                } else {
                    println!("  {} {}  (due: {})", marker, title, due);
                }
            }
            println!();
        }
        _ => {
            println!("No tasks in @{}", slug);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn task_markers() {
        let marker = |status: &str| match status {
            "done" => "✓",
            "in_progress" => "▲",
            _ => "○",
        };
        assert_eq!(marker("done"), "✓");
        assert_eq!(marker("in_progress"), "▲");
        assert_eq!(marker("open"), "○");
    }
}
