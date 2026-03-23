//! `os upgrade` — self-update the CLI binary from GitHub Releases.

use anyhow::{bail, Context, Result};
use semver::Version;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;

const GITHUB_REPO: &str = "agemalabs/os-cli";
const BINARY_NAME: &str = "os-aarch64-apple-darwin";

/// Parse a version tag like "v0.1.0" into a semver Version.
fn parse_version_tag(tag: &str) -> Result<Version> {
    let stripped = tag.strip_prefix('v').unwrap_or(tag);
    Version::parse(stripped).context(format!("Invalid version tag: {tag}"))
}

/// Verify SHA-256 checksum of a file against a checksums.txt content.
/// Format: "<hash>  <filename>\n"
fn verify_checksum(file_bytes: &[u8], checksums_content: &str, filename: &str) -> Result<()> {
    let expected_hash = checksums_content
        .lines()
        .find(|line| line.ends_with(filename))
        .and_then(|line| line.split_whitespace().next())
        .ok_or_else(|| anyhow::anyhow!("No checksum found for {filename}"))?;

    let mut hasher = Sha256::new();
    hasher.update(file_bytes);
    let actual_hash = format!("{:x}", hasher.finalize());

    if actual_hash != expected_hash {
        bail!(
            "Checksum mismatch for {filename}:\n  expected: {expected_hash}\n  actual:   {actual_hash}"
        );
    }

    Ok(())
}

/// Run the upgrade command.
pub async fn run() -> Result<()> {
    let current_version =
        Version::parse(env!("CARGO_PKG_VERSION")).context("Failed to parse current version")?;

    println!("Checking for updates...");

    let http = reqwest::Client::new();

    // Fetch latest release
    let release_url = format!(
        "https://api.github.com/repos/{GITHUB_REPO}/releases/latest"
    );
    let release_resp: serde_json::Value = http
        .get(&release_url)
        .header("User-Agent", "os-cli")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()
        .context("Failed to fetch latest release")?
        .json()
        .await?;

    let tag_name = release_resp["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No tag_name in release response"))?;

    let latest_version = parse_version_tag(tag_name)?;

    if latest_version <= current_version {
        println!("Already up to date (v{current_version}).");
        return Ok(());
    }

    // Download checksum file
    let checksum_url = format!(
        "https://github.com/{GITHUB_REPO}/releases/download/{tag_name}/checksums.txt"
    );
    let checksums_content = http
        .get(&checksum_url)
        .header("User-Agent", "os-cli")
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    // Download binary
    println!(
        "Upgrading os v{current_version} → v{latest_version}"
    );
    let binary_url = format!(
        "https://github.com/{GITHUB_REPO}/releases/download/{tag_name}/{BINARY_NAME}"
    );
    let binary_bytes = http
        .get(&binary_url)
        .header("User-Agent", "os-cli")
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    // Verify checksum
    verify_checksum(&binary_bytes, &checksums_content, BINARY_NAME)?;

    // Replace binary
    let current_exe = std::env::current_exe().context("Failed to determine current binary path")?;
    let current_exe = fs::canonicalize(&current_exe)
        .context("Failed to resolve canonical path of current binary")?;

    // Back up current binary
    let backup_path = current_exe.with_extension("bak");
    fs::rename(&current_exe, &backup_path)
        .context("Failed to back up current binary")?;

    // Write new binary
    let mut tmp = tempfile::NamedTempFile::new_in(
        current_exe
            .parent()
            .ok_or_else(|| anyhow::anyhow!("No parent directory for binary"))?,
    )?;
    tmp.write_all(&binary_bytes)?;

    // Set executable permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(tmp.path(), fs::Permissions::from_mode(0o755))?;
    }

    // Move into place (atomic on same filesystem)
    tmp.persist(&current_exe)
        .context("Failed to replace binary")?;

    println!("Updated. Restart your shell or run `os` again.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_tag_strips_v_prefix() {
        let v = parse_version_tag("v0.1.0").unwrap();
        assert_eq!(v, Version::new(0, 1, 0));
    }

    #[test]
    fn parse_version_tag_works_without_prefix() {
        let v = parse_version_tag("1.2.3").unwrap();
        assert_eq!(v, Version::new(1, 2, 3));
    }

    #[test]
    fn parse_version_tag_rejects_garbage() {
        assert!(parse_version_tag("not-a-version").is_err());
    }

    #[test]
    fn semver_comparison_is_correct() {
        let v1 = Version::new(0, 9, 0);
        let v2 = Version::new(0, 10, 0);
        assert!(v2 > v1, "0.10.0 should be greater than 0.9.0");
    }

    #[test]
    fn verify_checksum_passes_for_correct_hash() {
        let data = b"hello world";
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = format!("{:x}", hasher.finalize());

        let checksums = format!("{hash}  os-aarch64-apple-darwin\n");
        verify_checksum(data, &checksums, "os-aarch64-apple-darwin").unwrap();
    }

    #[test]
    fn verify_checksum_fails_for_wrong_hash() {
        let data = b"hello world";
        let checksums = "0000000000000000000000000000000000000000000000000000000000000000  os-aarch64-apple-darwin\n";
        assert!(verify_checksum(data, checksums, "os-aarch64-apple-darwin").is_err());
    }

    #[test]
    fn verify_checksum_fails_for_missing_filename() {
        let data = b"hello world";
        let checksums = "abc123  some-other-file\n";
        assert!(verify_checksum(data, checksums, "os-aarch64-apple-darwin").is_err());
    }
}
