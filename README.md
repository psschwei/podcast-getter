# podcast-getter

A lightweight CLI utility for downloading podcasts from RSS feeds. It tracks the last time each feed was checked and only downloads new episodes, with optional metadata tagging support.

## Features

- **Simple Configuration**: TOML-based config file stored in `~/.config/podcast-getter/`
- **Smart Tracking**: Remembers the last time each feed was checked and only downloads new episodes
- **Sequential Downloads**: Downloads episodes one at a time for simplicity and predictability
- **Metadata Tagging**: Optionally uses `audio-metadata` to tag downloaded files with podcast and episode information
- **Graceful Error Handling**: If one feed fails, continues with others and reports errors at the end
- **Multiple Feeds**: Support for multiple podcast feeds with different output directories

## Installation

### Prerequisites

- Rust 1.70+
- (Optional) [`audio-metadata`](https://github.com/psschwei/audio-metadata) for metadata tagging support

### Building

```bash
cargo build --release
# Binary will be at: target/release/podcast-getter
```

Optionally, install it to your system:

```bash
cargo install --path .
```

## Configuration

### Initialize Config

First, generate a config template:

```bash
podcast-getter init-config
```

This creates `/home/paul/.config/podcast-getter/config.toml.example` as a template.

### Edit Config

Create or edit `~/.config/podcast-getter/config.toml`:

```toml
[[podcasts]]
name = "Changelog"
url = "https://changelog.com/podcast/feed"
output_dir = "/home/paul/Downloads/podcasts/changelog"

[[podcasts]]
name = "CoRecursive"
url = "https://corecursive.com/feed/"
output_dir = "/home/paul/Downloads/podcasts/corecursive"
```

Each podcast needs:
- **name**: Display name for the podcast
- **url**: URL to the RSS feed
- **output_dir**: Directory where episodes will be saved

## Usage

### Download New Episodes

Download new episodes from all configured podcasts:

```bash
podcast-getter download
```

### Add a Podcast

Add a new podcast to your config:

```bash
podcast-getter add <FEED_URL> --name "Podcast Name" --output-dir /path/to/directory
```

### List Configured Podcasts

Show all configured podcasts and their settings:

```bash
podcast-getter list
```

### Show Status

Display last-check timestamps for all podcasts:

```bash
podcast-getter status
```

### Update Specific Feed

Download new episodes from a specific podcast:

```bash
podcast-getter update-feed "Podcast Name"
```

### Debug Logging

Enable debug logging for troubleshooting:

```bash
podcast-getter --debug download
```

## State Management

Downloaded episode information is stored in `~/.config/podcast-getter/state.json`. This file tracks:

- Last time each feed was checked
- Used to determine which episodes are "new"

The state file is created automatically on first successful download and updated after each check.

## Metadata Tagging (Optional)

If `audio-metadata` is installed and available in your PATH, `podcast-getter` will automatically tag downloaded files with:

- **Album**: Podcast name
- **Title**: Episode title

If `audio-metadata` is not found, the tool will log a warning and continue without tagging. This is entirely optional—the downloader works fine without it.

To install `audio-metadata`:

1. **Download from GitHub Releases** (Recommended):
   - Visit [audio-metadata releases](https://github.com/psschwei/audio-metadata/releases)
   - Download the binary for your platform
   - Extract and move it to a location in your PATH (e.g., `~/.local/bin/` or `/usr/local/bin/`)
   - Make it executable: `chmod +x audio-metadata`

2. **Build from source**:
   ```bash
   git clone https://github.com/psschwei/audio-metadata
   cd audio-metadata
   cargo install --path .
   ```

3. **Package manager** (if available for your system)

## Error Handling

If an error occurs while processing a feed:

1. That feed is skipped
2. Other feeds continue processing normally
3. A summary of errors is reported at the end

This graceful degradation means you'll still get episodes from working feeds even if one fails.

## Project Structure

```
src/
├── main.rs       - CLI entry point and argument parsing
├── config.rs     - Configuration file handling
├── state.rs      - State tracking (last-check timestamps)
├── feed.rs       - RSS feed parsing and filtering
├── download.rs   - File downloading
├── tagger.rs     - Metadata tagging via subprocess
└── cli.rs        - Command implementations
```

## License

MIT
