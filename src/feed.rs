use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rss::Channel;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Episode {
    pub title: String,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub url: String,
    pub pub_date: DateTime<Utc>,
}

/// Fetch and parse RSS feed
pub async fn fetch_feed(feed_url: &str) -> Result<(Vec<Episode>, Option<String>)> {
    let content = reqwest::get(feed_url)
        .await
        .context("Failed to fetch feed")?
        .text()
        .await
        .context("Failed to read feed content")?;

    let channel = Channel::from_str(&content)
        .context("Failed to parse RSS feed")?;

    let mut episodes = Vec::new();

    for item in channel.items() {
        // Try to extract URL from various possible locations
        let url = item
            .enclosure()
            .map(|e| e.url().to_string())
            .or_else(|| {
                item.link()
                    .map(|l| l.to_string())
            });

        if let Some(url) = url {
            let title = item
                .title()
                .map(|t| t.to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let description = item.description().map(|d| d.to_string());

            let pub_date = item
                .pub_date()
                .and_then(|date_str| {
                    // Try to parse RFC 2822 format (common in RSS)
                    DateTime::parse_from_rfc2822(date_str)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                })
                .unwrap_or_else(|| Utc::now());

            episodes.push(Episode {
                title,
                description,
                url,
                pub_date,
            });
        }
    }

    // Extract channel image URL
    let image_url = extract_channel_image(&channel);

    Ok((episodes, image_url))
}

/// Extract channel image URL from RSS feed
fn extract_channel_image(channel: &Channel) -> Option<String> {
    // Try to get image from channel.image()
    channel
        .image()
        .map(|img| img.url().to_string())
}

/// Filter episodes by publication date
pub fn filter_by_date(
    episodes: Vec<Episode>,
    since: Option<DateTime<Utc>>,
) -> Vec<Episode> {
    match since {
        None => episodes,
        Some(cutoff) => episodes
            .into_iter()
            .filter(|ep| ep.pub_date > cutoff)
            .collect(),
    }
}
