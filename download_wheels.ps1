#!/usr/bin/env pwsh
# Download Pillow wheels for all supported platforms
# This script downloads Pillow (PIL) wheels for Windows, macOS, and Linux

Write-Host "Downloading Pillow wheels for Blend Vault extension..." -ForegroundColor Green

# Create wheels directory if it doesn't exist
if (-not (Test-Path "wheels")) {
	New-Item -ItemType Directory -Path "wheels"
	Write-Host "Created wheels directory" -ForegroundColor Yellow
}

# Pillow version to download
$pillowVersion = "10.4.0"

# Download wheels for all platforms
Write-Host "Downloading Pillow $pillowVersion wheels..." -ForegroundColor Blue

# Windows x64
Write-Host "  - Windows x64..."
pip download "Pillow==$pillowVersion" --dest ./wheels --only-binary=:all: --python-version=3.11 --platform=win_amd64

# macOS ARM64 
Write-Host "  - macOS ARM64..."
pip download "Pillow==$pillowVersion" --dest ./wheels --only-binary=:all: --python-version=3.11 --platform=macosx_11_0_arm64

# Linux x64
Write-Host "  - Linux x64..."
pip download "Pillow==$pillowVersion" --dest ./wheels --only-binary=:all: --python-version=3.11 --platform=manylinux_2_28_x86_64

Write-Host "Wheel download complete!" -ForegroundColor Green
Write-Host "Downloaded wheels:" -ForegroundColor Cyan
Get-ChildItem wheels/*.whl | ForEach-Object { Write-Host "  $($_.Name)" }
