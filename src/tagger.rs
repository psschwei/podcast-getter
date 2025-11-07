use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Try to tag audio file with metadata using audio-metadata
pub fn tag_audio_file(
    file_path: &Path,
    podcast_name: &str,
    episode_title: &str,
    cover_art_path: Option<&Path>,
) -> Result<()> {
    // Check if audio-metadata is available
    match Command::new("audio-metadata")
        .arg("--help")
        .output()
    {
        Ok(_) => {
            // audio-metadata is available, use it to tag the file
            run_audio_metadata(file_path, podcast_name, episode_title, cover_art_path)
        }
        Err(_) => {
            // audio-metadata not found, log warning and skip
            tracing::warn!(
                "audio-metadata not found in PATH. Skipping metadata tagging for {}",
                file_path.display()
            );
            Ok(())
        }
    }
}

fn run_audio_metadata(
    file_path: &Path,
    podcast_name: &str,
    episode_title: &str,
    cover_art_path: Option<&Path>,
) -> Result<()> {
    let mut cmd = Command::new("audio-metadata");
    cmd.arg("set")
        .arg("-f")
        .arg(file_path)
        .arg("--album")
        .arg(podcast_name)
        .arg("--artist")
        .arg(podcast_name)
        .arg("--title")
        .arg(episode_title);

    // Add cover art if provided
    if let Some(cover_path) = cover_art_path {
        cmd.arg("-c").arg(cover_path);
    }

    let output = cmd.output().context("Failed to run audio-metadata")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(
            "audio-metadata failed for {}: {}",
            file_path.display(),
            stderr
        );
    } else {
        let cover_info = if cover_art_path.is_some() {
            " with cover art"
        } else {
            ""
        };
        tracing::debug!(
            "Tagged {} with podcast '{}' and episode '{}'{}",
            file_path.display(),
            podcast_name,
            episode_title,
            cover_info
        );
    }

    Ok(())
}
