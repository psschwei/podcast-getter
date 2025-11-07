use anyhow::{Context, Result};
use image::io::Reader as ImageReader;
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// Download and convert image to PNG, returning path to the saved image
pub async fn download_and_convert_image(
    image_url: &str,
    output_dir: &Path,
    podcast_name: &str,
) -> Result<PathBuf> {
    // Download the image
    let image_data = reqwest::get(image_url)
        .await
        .context("Failed to download image")?
        .bytes()
        .await
        .context("Failed to read image content")?;

    // Parse the image to validate and potentially convert it
    let img = ImageReader::new(Cursor::new(&image_data))
        .with_guessed_format()
        .context("Failed to read image data")?
        .decode()
        .context("Failed to decode image")?;

    // Convert to PNG and save
    let filename = format!("{}_cover.png", sanitize_filename(podcast_name));
    let file_path = output_dir.join(&filename);

    img.save_with_format(&file_path, image::ImageFormat::Png)
        .context("Failed to save image as PNG")?;

    tracing::debug!(
        "Downloaded and converted image for '{}' to {}",
        podcast_name,
        file_path.display()
    );

    Ok(file_path)
}

/// Sanitize podcast name for use in filename
fn sanitize_filename(name: &str) -> String {
    name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
