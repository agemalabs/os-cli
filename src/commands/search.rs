//! `os search` — semantic search across projects.

use crate::api_client::ApiClient;
use anyhow::Result;

/// Run a semantic search and print results.
pub async fn run(client: &ApiClient, query: &str, project: Option<&str>) -> Result<()> {
    let body = serde_json::json!({
        "query": query,
        "project_slug": project,
        "limit": 5
    });

    let resp: serde_json::Value = client.post("/search", &body).await?;
    let results = resp["data"].as_array();

    match results {
        Some(results) if !results.is_empty() => {
            for result in results {
                let similarity = result["similarity"].as_f64().unwrap_or(0.0);
                let project_slug = result["project_slug"].as_str().unwrap_or("?");
                let file_slug = result["file_slug"].as_str().unwrap_or("?");
                let chunk = result["chunk_text"].as_str().unwrap_or("");

                println!("\n  {} / {}  ({:.2})", project_slug, file_slug, similarity);
                // Show first 200 chars of chunk
                let preview = if chunk.len() > 200 {
                    format!("{}...", &chunk[..200])
                } else {
                    chunk.to_string()
                };
                println!("    {}", preview.replace('\n', "\n    "));
            }
            println!();
        }
        _ => {
            println!("No results found.");
        }
    }

    Ok(())
}
