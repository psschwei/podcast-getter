use crate::config::{Config, PodcastConfig};
use crate::download;
use crate::feed;
use crate::image;
use crate::state::State;
use crate::tagger;
use anyhow::{bail, Result};
use chrono::Utc;
use std::path::PathBuf;
use tracing::info;

pub async fn download_all_podcasts(max_episodes: Option<usize>) -> Result<()> {
    info!("Starting podcast download");

    let config = Config::load()?;
    let mut state = State::load()?;

    let mut errors = Vec::new();

    for podcast in config.podcasts {
        if podcast.paused {
            info!("Skipping '{}' (paused)", podcast.name);
            continue;
        }
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
            Ok((file_path, prefixed_title)) => {
                // Try to tag the file with cover art if available
                if let Err(e) = tagger::tag_audio_file(&file_path, &podcast.name, &prefixed_title, cover_art_path.as_deref())
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

async fn download_episode(podcast: &PodcastConfig, episode: &feed::Episode) -> Result<(PathBuf, String)> {
    // Extract file extension from URL
    let extension = extract_extension(&episode.url).unwrap_or("mp3");

    // Prefix title with publication date for chronological sorting
    let date_prefix = episode.pub_date.format("%Y-%m-%d");
    let prefixed_title = format!("{} {}", date_prefix, episode.title);

    // Generate filename
    let filename = download::generate_filename(&prefixed_title, extension);
    let file_path = podcast.output_dir.join(&filename);

    info!(
        "Downloading '{}' to {}",
        episode.title,
        file_path.display()
    );

    download::download_file(&episode.url, &file_path).await?;

    Ok((file_path, prefixed_title))
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
        paused: false,
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
        let paused_indicator = if podcast.paused { " (paused)" } else { "" };
        println!("Name: {}{}", podcast.name, paused_indicator);
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
        let paused_indicator = if podcast.paused { " (paused)" } else { "" };
        match state.get_last_check(&podcast.name) {
            Some(last_check) => {
                println!(
                    "{}{}: last checked {}",
                    podcast.name,
                    paused_indicator,
                    last_check.format("%Y-%m-%d %H:%M:%S UTC")
                );
            }
            None => {
                println!("{}{}: never checked", podcast.name, paused_indicator);
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

    if podcast.paused {
        tracing::warn!("Podcast '{}' is paused, but updating anyway since it was explicitly requested", podcast.name);
    }

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

pub fn clean_podcasts() -> Result<()> {
    let config = Config::load()?;

    if config.podcasts.is_empty() {
        println!("No podcasts configured.");
        return Ok(());
    }

    let mut total_deleted = 0;
    let mut errors = Vec::new();

    println!("Cleaning MP3 files from configured podcast directories...\n");

    for podcast in &config.podcasts {
        let output_dir = &podcast.output_dir;

        if !output_dir.exists() {
            println!("Skipping '{}': directory does not exist", podcast.name);
            continue;
        }

        match std::fs::read_dir(output_dir) {
            Ok(entries) => {
                let mut podcast_deleted = 0;

                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            if path.is_file() {
                                if let Some(extension) = path.extension() {
                                    if extension.to_string_lossy().to_lowercase() == "mp3" {
                                        match std::fs::remove_file(&path) {
                                            Ok(_) => {
                                                podcast_deleted += 1;
                                                total_deleted += 1;
                                            }
                                            Err(e) => {
                                                let error_msg = format!(
                                                    "Failed to delete {}: {}",
                                                    path.display(),
                                                    e
                                                );
                                                tracing::warn!("{}", error_msg);
                                                errors.push(error_msg);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let error_msg = format!(
                                "Failed to read entry in {}: {}",
                                output_dir.display(),
                                e
                            );
                            tracing::warn!("{}", error_msg);
                            errors.push(error_msg);
                        }
                    }
                }

                if podcast_deleted > 0 {
                    println!(
                        "Deleted {} MP3 file{} from '{}'",
                        podcast_deleted,
                        if podcast_deleted == 1 { "" } else { "s" },
                        podcast.name
                    );
                }
            }
            Err(e) => {
                let error_msg = format!(
                    "Failed to read directory for '{}': {}",
                    podcast.name, e
                );
                tracing::error!("{}", error_msg);
                errors.push(error_msg);
            }
        }
    }

    println!(
        "\nTotal: deleted {} MP3 file{}",
        total_deleted,
        if total_deleted == 1 { "" } else { "s" }
    );

    // Report errors if any
    if !errors.is_empty() {
        println!("\nErrors encountered:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    Ok(())
}

pub fn print_podcast_names() -> Result<()> {
    let config = Config::load().unwrap_or(Config { podcasts: vec![] });
    for podcast in &config.podcasts {
        println!("{}", podcast.name);
    }
    Ok(())
}

pub fn print_completions(shell: &str) -> Result<()> {
    match shell {
        "bash" => print!("{}", BASH_COMPLETION),
        "zsh" => print!("{}", ZSH_COMPLETION),
        "fish" => print!("{}", FISH_COMPLETION),
        other => bail!("Unsupported shell: {}. Supported: bash, zsh, fish", other),
    }
    Ok(())
}

const BASH_COMPLETION: &str = r#"_pg() {
    local cur prev words cword
    _init_completion || return

    local subcommands="download add list status update-feed init-config clean pause unpause completions"

    if [[ $cword -eq 1 ]]; then
        COMPREPLY=($(compgen -W "$subcommands" -- "$cur"))
        return
    fi

    case "${words[1]}" in
        update-feed|pause|unpause)
            local names
            names=$(pg names 2>/dev/null)
            COMPREPLY=($(compgen -W "$names" -- "$cur"))
            ;;
        download)
            case "$prev" in
                -m|--max-episodes) return ;;
            esac
            COMPREPLY=($(compgen -W "--max-episodes -m --debug -d" -- "$cur"))
            ;;
        add)
            case "$prev" in
                -n|--name|-o|--output-dir) return ;;
            esac
            COMPREPLY=($(compgen -W "--name -n --output-dir -o --debug -d" -- "$cur"))
            ;;
        completions)
            COMPREPLY=($(compgen -W "bash zsh fish" -- "$cur"))
            ;;
    esac
}

complete -F _pg pg
"#;

const ZSH_COMPLETION: &str = r#"#compdef pg

_pg() {
    local state

    _arguments \
        '(-d --debug)'{-d,--debug}'[Enable debug logging]' \
        ':command:->command' \
        '*::args:->args'

    case $state in
        command)
            local commands=(
                'download:Download new episodes from all configured podcasts'
                'add:Add a new podcast to the config'
                'list:List all configured podcasts'
                'status:Show last-check timestamps for all podcasts'
                'update-feed:Check and download new episodes from a specific podcast'
                'init-config:Generate an example config file'
                'clean:Remove all downloaded MP3 files from configured podcasts'
                'pause:Pause a podcast so it is skipped during download'
                'unpause:Unpause a podcast so it resumes downloading'
                'completions:Generate shell completion scripts'
            )
            _describe 'command' commands
            ;;
        args)
            case $words[1] in
                update-feed|pause|unpause)
                    local names=(${(f)"$(pg names 2>/dev/null)"})
                    _describe 'podcast' names
                    ;;
                completions)
                    local shells=('bash' 'zsh' 'fish')
                    _describe 'shell' shells
                    ;;
                download)
                    _arguments \
                        '(-m --max-episodes)'{-m,--max-episodes}'[Maximum episodes per podcast]:count'
                    ;;
                add)
                    _arguments \
                        '(-n --name)'{-n,--name}'[Name for the podcast]:name' \
                        '(-o --output-dir)'{-o,--output-dir}'[Output directory]:directory:_files -/'
                    ;;
            esac
            ;;
    esac
}

_pg
"#;

const FISH_COMPLETION: &str = r#"function __pg_podcast_names
    pg names 2>/dev/null
end

complete -c pg -f

complete -c pg -n '__fish_use_subcommand' -a download -d 'Download new episodes from all configured podcasts'
complete -c pg -n '__fish_use_subcommand' -a add -d 'Add a new podcast to the config'
complete -c pg -n '__fish_use_subcommand' -a list -d 'List all configured podcasts'
complete -c pg -n '__fish_use_subcommand' -a status -d 'Show last-check timestamps for all podcasts'
complete -c pg -n '__fish_use_subcommand' -a update-feed -d 'Check and download new episodes from a specific podcast'
complete -c pg -n '__fish_use_subcommand' -a init-config -d 'Generate an example config file'
complete -c pg -n '__fish_use_subcommand' -a clean -d 'Remove all downloaded MP3 files from configured podcasts'
complete -c pg -n '__fish_use_subcommand' -a pause -d 'Pause a podcast so it is skipped during download'
complete -c pg -n '__fish_use_subcommand' -a unpause -d 'Unpause a podcast so it resumes downloading'
complete -c pg -n '__fish_use_subcommand' -a completions -d 'Generate shell completion scripts'

complete -c pg -n '__fish_seen_subcommand_from update-feed pause unpause' -a '(__pg_podcast_names)'
complete -c pg -n '__fish_seen_subcommand_from completions' -a 'bash zsh fish'

complete -c pg -s d -l debug -d 'Enable debug logging'
complete -c pg -n '__fish_seen_subcommand_from download' -s m -l max-episodes -d 'Maximum episodes per podcast' -r
complete -c pg -n '__fish_seen_subcommand_from add' -s n -l name -d 'Name for the podcast' -r
complete -c pg -n '__fish_seen_subcommand_from add' -s o -l output-dir -d 'Output directory' -r
complete -c pg -n '__fish_seen_subcommand_from pause unpause' -l all -d 'Apply to all podcasts'
"#;

pub fn pause_podcast(name: Option<String>, all: bool) -> Result<()> {
    if !all && name.is_none() {
        anyhow::bail!("Either provide a podcast name or use --all");
    }

    let mut config = Config::load()?;

    if all {
        let mut count = 0;
        for podcast in &mut config.podcasts {
            if !podcast.paused {
                podcast.paused = true;
                count += 1;
            }
        }
        config.save()?;
        println!("Paused {} podcast{}", count, if count == 1 { "" } else { "s" });
    } else {
        let name = name.unwrap();
        let podcast = config
            .podcasts
            .iter_mut()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow::anyhow!("Podcast '{}' not found in config", name))?;

        if podcast.paused {
            println!("Podcast '{}' is already paused", name);
        } else {
            podcast.paused = true;
            config.save()?;
            println!("Paused '{}'", name);
        }
    }

    Ok(())
}

pub fn unpause_podcast(name: Option<String>, all: bool) -> Result<()> {
    if !all && name.is_none() {
        anyhow::bail!("Either provide a podcast name or use --all");
    }

    let mut config = Config::load()?;

    if all {
        let mut count = 0;
        for podcast in &mut config.podcasts {
            if podcast.paused {
                podcast.paused = false;
                count += 1;
            }
        }
        config.save()?;
        println!("Unpaused {} podcast{}", count, if count == 1 { "" } else { "s" });
    } else {
        let name = name.unwrap();
        let podcast = config
            .podcasts
            .iter_mut()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow::anyhow!("Podcast '{}' not found in config", name))?;

        if !podcast.paused {
            println!("Podcast '{}' is not paused", name);
        } else {
            podcast.paused = false;
            config.save()?;
            println!("Unpaused '{}'", name);
        }
    }

    Ok(())
}
