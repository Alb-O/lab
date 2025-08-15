use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use reqwest::header::CONTENT_LENGTH;
use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

// Map format: { "folder": { "relative/path.ext": "https://..." } }
// We'll deserialize to BTreeMap<String, BTreeMap<String, String>> for deterministic ordering.

pub type FolderMap = BTreeMap<String, BTreeMap<String, String>>;

pub fn read_map(path: &Path) -> Result<FolderMap> {
    let data = fs::read(path).with_context(|| format!("read map: {}", path.display()))?;
    let map: FolderMap = serde_json::from_slice(&data).with_context(|| "parse JSON map")?;
    Ok(map)
}

pub fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("create dir: {}", dir.display()))?;
    }
    Ok(())
}

pub fn filename_from_url(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()?
        .path_segments()?
        .next_back()
        .map(|s| s.to_string())
}

pub fn download_with_progress(client: &Client, url: &str, dest: &Path) -> Result<()> {
    // HEAD to get length if available
    let mut total: Option<u64> = None;
    if let Ok(resp) = client.head(url).send() {
        if let Some(len) = resp.headers().get(CONTENT_LENGTH) {
            if let Ok(s) = len.to_str() {
                total = s.parse::<u64>().ok();
            }
        }
    }

    let pb = if let Some(total) = total {
        ProgressBar::new(total)
    } else {
        ProgressBar::new_spinner()
    };
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} {bytes:>8}/{total_bytes:8} {wide_msg}")
            .unwrap(),
    );
    pb.set_message(url.to_string());

    // Stream download
    let mut resp = client
        .get(url)
        .send()
        .with_context(|| format!("GET {url}"))?
        .error_for_status()
        .with_context(|| format!("non-success status from {url}"))?;
    let mut file = File::create(dest).with_context(|| format!("create {}", dest.display()))?;

    let mut buf = [0u8; 64 * 1024];
    let mut downloaded: u64 = 0;
    loop {
        let n = resp.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        downloaded += n as u64;
        pb.set_position(downloaded);
    }
    pb.finish_with_message(format!("saved to {}", dest.display()));
    Ok(())
}

pub struct DownloaderConfig {
    pub root: PathBuf,
    pub map_path: Option<PathBuf>,
    pub force: bool,
    pub folder: Option<String>,
    pub dry_run: bool,
}

impl Default for DownloaderConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("blendfiles"),
            map_path: None,
            force: false,
            folder: None,
            dry_run: false,
        }
    }
}

pub fn run_downloader(config: DownloaderConfig) -> Result<()> {
    // Resolve map path relative to workspace root if needed
    let root = if config.root.is_absolute() {
        config.root.clone()
    } else {
        // Assume running from repo root; fall back to current dir
        PathBuf::from(std::env::var("CARGO_WORKSPACE_DIR").unwrap_or_else(|_| String::from(".")))
            .join(config.root)
    };

    let map_path = config
        .map_path
        .unwrap_or_else(|| root.join("blendfiles_map.json"));

    if !map_path.exists() {
        anyhow::bail!("map file not found: {}", map_path.display());
    }

    let map = read_map(&map_path)?;

    let client = Client::builder()
        .user_agent("dot001-dev-downloader/0.1")
        .build()?;

    for (folder, files) in map {
        if let Some(ref only) = config.folder {
            if &folder != only {
                continue;
            }
        }
        for (rel, url) in files {
            let mut dest_rel = PathBuf::from(&folder);
            for part in rel.split(&['/', '\\'][..]).filter(|p| !p.is_empty()) {
                dest_rel.push(part);
            }
            let dest = root.join(&dest_rel);
            ensure_parent_dir(&dest)?;
            if dest.exists() && !config.force {
                println!("{} already exists, skipping.", dest_rel.display());
                continue;
            }
            if config.dry_run {
                println!("Would download {} from {}", dest_rel.display(), url);
                continue;
            }
            println!("Downloading {}...", dest_rel.display());
            if let Some(name) = filename_from_url(&url) {
                // Ensure trailing path segment matches file name; not strictly required
                let _ = name;
            }
            if let Err(err) = download_with_progress(&client, &url, &dest) {
                eprintln!(
                    "Failed to download {} -> {}: {:#}",
                    url,
                    dest.display(),
                    err
                );
            }
        }
    }

    Ok(())
}
