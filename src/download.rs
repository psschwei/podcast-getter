use anyhow::{Context, Result};
use std::path::Path;

/// Download a file from a URL and save it to disk
pub async fn download_file(url: &str, output_path: &Path) -> Result<()> {
    // Create parent directories if they don't exist
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .context("Failed to create output directory")?;
    }

    let response = reqwest::get(url)
        .await
        .context("Failed to fetch file")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to download file: HTTP {}",
            response.status()
        );
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read response body")?;

    std::fs::write(output_path, bytes)
        .context("Failed to write file to disk")?;

    Ok(())
}

/// Generate a filename from an episode title
pub fn generate_filename(title: &str, extension: &str) -> String {
    // Remove invalid characters and limit length
    let sanitized = title
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
        .collect::<String>();

    let trimmed = sanitized.trim();
    let limited = if trimmed.len() > 100 {
        &trimmed[..100]
    } else {
        trimmed
    };

    format!("{}.{}", limited.trim(), extension)
}
