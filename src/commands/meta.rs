//! `os meta` — print project meta file.

use crate::api_client::ApiClient;
use anyhow::Result;

/// Fetch and display a project's _meta.md content.
pub async fn run(client: &ApiClient, project: &str) -> Result<()> {
    let slug = project.trim_start_matches('@');
    let path = format!("/projects/{}/meta", slug);
    let resp: serde_json::Value = client.get(&path).await?;

    if let Some(content) = resp["data"]["content"].as_str() {
        println!("{}", content);
    } else {
        println!("No meta file found for project '{}'", slug);
    }

    Ok(())
}
