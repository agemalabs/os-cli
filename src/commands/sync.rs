//! `os sync --watch @<project>` — watch and sync files on change.

use crate::api_client::ApiClient;
use crate::commands::push;
use anyhow::Result;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;

/// Sync a directory with a project, optionally watching for changes.
pub async fn run(client: &ApiClient, project: &str, watch: bool) -> Result<()> {
    let slug = project.trim_start_matches('@');

    if !watch {
        // One-time sync: push all files in current directory
        push::run(client, ".", project, None).await?;
        return Ok(());
    }

    // Watch mode
    println!("Watching for changes to sync with @{}...", slug);
    println!("Press Ctrl+C to stop.\n");

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(Path::new("."), RecursiveMode::Recursive)?;

    // Process file change events
    for event in rx {
        match event {
            Ok(event) => {
                if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                    for path in &event.paths {
                        // Skip hidden files and directories
                        if path
                            .components()
                            .any(|c| c.as_os_str().to_str().is_some_and(|s| s.starts_with('.')))
                        {
                            continue;
                        }

                        if path.is_file() {
                            let path_str = path.to_str().unwrap_or("");
                            println!("  Changed: {}", path_str);
                            if let Err(e) = push::run(client, path_str, project, None).await {
                                eprintln!("  Error pushing {}: {}", path_str, e);
                            }
                        }
                    }
                }
            }
            Err(e) => eprintln!("Watch error: {}", e),
        }
    }

    Ok(())
}
