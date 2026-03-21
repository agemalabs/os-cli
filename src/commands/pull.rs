//! `os pull @<project>` — pull latest files from a project.

use crate::api_client::ApiClient;
use anyhow::Result;
use std::fs;
use std::path::Path;

/// Pull all files from a project to a local directory.
pub async fn run(client: &ApiClient, project: &str) -> Result<()> {
    let slug = project.trim_start_matches('@');

    let resp: serde_json::Value = client.get(&format!("/projects/{}/files", slug)).await?;
    let files = resp["data"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Unexpected response format"))?;

    if files.is_empty() {
        println!("No files in @{}", slug);
        return Ok(());
    }

    // Create output directory
    let dir = Path::new(slug);
    fs::create_dir_all(dir)?;

    let mut count = 0;
    for file in files {
        let file_slug = file["slug"].as_str().unwrap_or("unnamed");
        let content = file["content"].as_str().unwrap_or("");

        let filename = format!("{}.md", file_slug);
        let filepath = dir.join(&filename);

        fs::write(&filepath, content)?;
        println!("  {} → {}", file_slug, filepath.display());
        count += 1;
    }

    println!("\nPulled {} files from @{}", count, slug);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn slug_strips_at_prefix() {
        let slug = "@brownells".trim_start_matches('@');
        assert_eq!(slug, "brownells");
    }

    #[test]
    fn slug_without_prefix_unchanged() {
        let slug = "brownells".trim_start_matches('@');
        assert_eq!(slug, "brownells");
    }
}
