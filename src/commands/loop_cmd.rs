//! `os status` — daily briefing.

use crate::api_client::ApiClient;
use anyhow::Result;

/// Fetch and display the daily status briefing.
pub async fn run(client: &ApiClient, json: bool) -> Result<()> {
    let resp: serde_json::Value = client.get("/status").await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&resp)?);
    } else {
        // Display the AI-generated summary
        if let Some(summary) = resp["data"]["generated_summary"].as_str() {
            println!("\n{}\n", summary);
        }

        // Display tasks
        if let Some(tasks) = resp["data"]["my_tasks"].as_array() {
            if !tasks.is_empty() {
                println!("YOUR TASKS");
                for task in tasks {
                    let title = task["title"].as_str().unwrap_or("?");
                    let status = task["status"].as_str().unwrap_or("?");
                    let marker = match status {
                        "done" => "✓",
                        "in_progress" => "▲",
                        _ => "○",
                    };
                    println!("  {} {}", marker, title);
                }
                println!();
            }
        }
    }

    Ok(())
}
