blendfiles_downloader
=====================

Cross-platform utility to download sample .blend files referenced by `blendfiles/blendfiles_map.json`.

Usage
-----

- Build and run:
  - cargo run -p dot001_blendfiles_downloader -- --root blendfiles
- Options:
  - --root <DIR>    Root directory containing `blendfiles_map.json` and download destination (default: blendfiles)
  - --map <FILE>    Path to map JSON (defaults to <root>/blendfiles_map.json)
  - --folder <NAME> Only process a specific folder key
  - --force         Re-download even if file exists
  - --dry-run       Print planned downloads without fetching

Notes
-----

- Uses reqwest blocking client with rustls TLS backend; no native OpenSSL needed.
- Creates directories as needed; skips existing files unless `--force` is set.
