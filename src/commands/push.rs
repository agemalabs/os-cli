//! `os push <path> @<project>` — push file or directory to a project.

use crate::api_client::ApiClient;
use anyhow::{Context, Result};
use std::path::Path;

/// Push a file or directory to a project.
pub async fn run(
    client: &ApiClient,
    path: &str,
    project: &str,
    category: Option<&str>,
) -> Result<()> {
    let slug = project.trim_start_matches('@');
    let path = Path::new(path);

    if !path.exists() {
        anyhow::bail!("Path not found: {}", path.display());
    }

    if path.is_dir() {
        push_directory(client, path, slug, category).await
    } else {
        push_file(client, path, slug, category).await
    }
}

/// Push a single file.
async fn push_file(
    client: &ApiClient,
    path: &Path,
    project_slug: &str,
    category: Option<&str>,
) -> Result<()> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unnamed");

    let content =
        std::fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;

    // Determine if it's a markdown/text file or binary
    let is_text = matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md" | "txt" | "csv" | "json" | "yaml" | "yml" | "toml")
    );

    if is_text {
        // Push as a file (markdown content)
        let text = String::from_utf8(content)
            .with_context(|| format!("File is not valid UTF-8: {}", path.display()))?;

        let file_slug = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed");

        let body = serde_json::json!({
            "title": filename,
            "slug": file_slug,
            "category": category.unwrap_or("other"),
            "content": text,
        });

        let resp: serde_json::Value = client
            .post(&format!("/projects/{}/files", project_slug), &body)
            .await?;

        let slug = resp["data"]["slug"].as_str().unwrap_or("?");
        println!("  Pushed {} → @{}/{}", filename, project_slug, slug);
    } else {
        // Push as a document (binary upload)
        let mime = mime_from_extension(path);
        client
            .upload_file(
                &format!("/projects/{}/documents", project_slug),
                filename,
                &mime,
                content,
            )
            .await?;

        println!("  Uploaded {} → @{}", filename, project_slug);
    }

    Ok(())
}

/// Push all files in a directory.
async fn push_directory(
    client: &ApiClient,
    dir: &Path,
    project_slug: &str,
    category: Option<&str>,
) -> Result<()> {
    let mut count = 0;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            push_file(client, &path, project_slug, category).await?;
            count += 1;
        }
    }

    println!("\nPushed {} files to @{}", count, project_slug);
    Ok(())
}

/// Guess MIME type from file extension.
fn mime_from_extension(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("pdf") => "application/pdf",
        Some("docx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        Some("xlsx") => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("zip") => "application/zip",
        Some("csv") => "text/csv",
        _ => "application/octet-stream",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn mime_detection() {
        assert_eq!(
            mime_from_extension(Path::new("spec.pdf")),
            "application/pdf"
        );
        assert_eq!(mime_from_extension(Path::new("photo.png")), "image/png");
        assert_eq!(
            mime_from_extension(Path::new("unknown.xyz")),
            "application/octet-stream"
        );
    }
}
