use crate::config::{Config, PodcastConfig};
use crate::download;
use crate::feed;
use crate::image;
use crate::state::State;
use crate::tagger;
use anyhow::Result;
use chrono::Utc;
use std::path::PathBuf;
use tracing::info;

pub async fn download_all_podcasts(max_episodes: Option<usize>) -> Result<()> {
    info!("Starting podcast download");

    let config = Config::load()?;
    let mut state = State::load()?;

    let mut errors = Vec::new();

    for podcast in config.podcasts {
        match download_podcast(&podcast, &mut state, max_episodes).await {
            Ok(count) => {
                info!("Downloaded {} new episodes from {}", count, podcast.name);
            }
            Err(e) => {
                let error_msg = format!("Failed to process {}: {}", podcast.name, e);
                tracing::error!("{}", error_msg);
                errors.push(error_msg);
            }
        }
    }

    // Save updated state
    state.save()?;

    // Report errors
    if !errors.is_empty() {
        println!("\n⚠️  Errors encountered while processing feeds:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    info!("Podcast download complete");
    Ok(())
}

async fn download_podcast(podcast: &PodcastConfig, state: &mut State, max_episodes: Option<usize>) -> Result<usize> {
    let last_check = state.get_last_check(&podcast.name);

    // Fetch and parse feed
    let (episodes, image_url) = feed::fetch_feed(&podcast.url).await?;

    // Download and cache the cover art if available
    let cover_art_path = match image_url {
        Some(url) => {
            match image::download_and_convert_image(&url, &podcast.output_dir, &podcast.name).await {
                Ok(path) => {
                    info!("Downloaded cover art for '{}'", podcast.name);
                    Some(path)
                }
                Err(e) => {
                    tracing::warn!("Failed to download cover art for '{}': {}", podcast.name, e);
                    None
                }
            }
        }
        None => {
            info!("No cover art found for podcast '{}'", podcast.name);
            None
        }
    };

    // Filter by date
    let mut new_episodes = feed::filter_by_date(episodes, last_check);

    // Apply max_episodes limit from CLI or config
    let limit = max_episodes.or(podcast.max_episodes);
    if let Some(max) = limit {
        new_episodes.truncate(max);
    }

    if new_episodes.is_empty() {
        info!("No new episodes for {}", podcast.name);
        return Ok(0);
    }

    info!("Found {} new episodes for {}", new_episodes.len(), podcast.name);

    let mut downloaded = 0;
    for episode in new_episodes {
        match download_episode(&podcast, &episode).await {
            Ok(file_path) => {
                // Try to tag the file with cover art if available
                if let Err(e) = tagger::tag_audio_file(&file_path, &podcast.name, &episode.title, cover_art_path.as_deref())
                {
                    tracing::warn!("Failed to tag file {}: {}", file_path.display(), e);
                }
                downloaded += 1;
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to download episode '{}': {}",
                    episode.title,
                    e
                );
            }
        }
    }

    // Update last check time
    state.set_last_check(podcast.name.clone(), Utc::now());

    Ok(downloaded)
}

async fn download_episode(podcast: &PodcastConfig, episode: &feed::Episode) -> Result<PathBuf> {
    // Extract file extension from URL
    let extension = extract_extension(&episode.url).unwrap_or("mp3");

    // Generate filename
    let filename = download::generate_filename(&episode.title, extension);
    let file_path = podcast.output_dir.join(&filename);

    info!(
        "Downloading '{}' to {}",
        episode.title,
        file_path.display()
    );

    download::download_file(&episode.url, &file_path).await?;

    Ok(file_path)
}

fn extract_extension(url: &str) -> Option<&str> {
    url.split('.')
        .last()
        .and_then(|part| {
            // Take only the part before any query string
            part.split('?').next()
        })
        .and_then(|ext| {
            // Validate it looks like a real extension
            if ext.len() <= 10 && ext.chars().all(|c| c.is_alphanumeric()) {
                Some(ext)
            } else {
                None
            }
        })
}

pub fn add_podcast(url: String, name: Option<String>, output_dir: Option<PathBuf>) -> Result<()> {
    let mut config = Config::load().unwrap_or(Config { podcasts: vec![] });

    let podcast_name = name.unwrap_or_else(|| {
        // Try to extract name from URL
        url.split('/')
            .find(|part| !part.is_empty())
            .unwrap_or("podcast")
            .to_string()
    });

    let output = output_dir.unwrap_or_else(|| {
        dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("podcasts")
    });

    config.podcasts.push(PodcastConfig {
        name: podcast_name.clone(),
        url,
        output_dir: output,
        max_episodes: None,
    });

    config.save()?;

    println!("Added podcast '{}' to config", podcast_name);
    Ok(())
}

pub fn list_podcasts() -> Result<()> {
    let config = Config::load()?;
    let state = State::load()?;

    if config.podcasts.is_empty() {
        println!("No podcasts configured.");
        return Ok(());
    }

    println!("Configured podcasts:\n");

    for podcast in &config.podcasts {
        println!("Name: {}", podcast.name);
        println!("URL: {}", podcast.url);
        println!("Output: {}", podcast.output_dir.display());

        if let Some(last_check) = state.get_last_check(&podcast.name) {
            println!("Last checked: {}", last_check.format("%Y-%m-%d %H:%M:%S UTC"));
        } else {
            println!("Last checked: never");
        }

        println!();
    }

    Ok(())
}

pub fn show_status() -> Result<()> {
    let config = Config::load()?;
    let state = State::load()?;

    if config.podcasts.is_empty() {
        println!("No podcasts configured.");
        return Ok(());
    }

    println!("Podcast Status:\n");

    for podcast in &config.podcasts {
        match state.get_last_check(&podcast.name) {
            Some(last_check) => {
                println!(
                    "{}: last checked {}",
                    podcast.name,
                    last_check.format("%Y-%m-%d %H:%M:%S UTC")
                );
            }
            None => {
                println!("{}: never checked", podcast.name);
            }
        }
    }

    Ok(())
}

pub async fn update_feed(podcast_name: String) -> Result<()> {
    let config = Config::load()?;

    let podcast = config
        .podcasts
        .iter()
        .find(|p| p.name == podcast_name)
        .ok_or_else(|| {
            anyhow::anyhow!("Podcast '{}' not found in config", podcast_name)
        })?;

    let mut state = State::load()?;

    match download_podcast(podcast, &mut state, None).await {
        Ok(count) => {
            info!("Downloaded {} new episodes from {}", count, podcast.name);
            state.save()?;
        }
        Err(e) => {
            anyhow::bail!("Failed to update feed: {}", e);
        }
    }

    Ok(())
}
