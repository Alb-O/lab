# Testing the Build Manager Fix

## Issue
When refreshing the list of Blender builds (scrape again), installed versions would disappear from the UI completely and only reappear after a reload.

## Root Cause
The `getCachedBuilds()` method in `BuildManager.ts` was only handling orphaned builds (builds in the installed cache but not in the official scraped cache), but it wasn't updating the install status of builds that existed in both caches.

## Fix Applied
Updated the `getCachedBuilds()` method to:

1. **First**: For each build in the official scraped builds, check if it exists in the installed builds cache
2. **If it exists**: Update the build's install-related properties (`extractedPath`, `archivePath`, `customExecutable`)
3. **Then**: Handle orphaned builds as before

## Code Changes

### BuildManager.ts
- Enhanced `getCachedBuilds()` method to properly merge installed build metadata with official builds
- Added loop to update official builds with their installation status before handling orphaned builds

### BlenderViewRenderer.ts  
- Cleaned up unused `cachedBuilds` property that was being set but never used
- Simplified `onBuildsUpdated()` handler to just refresh the UI

## How to Test
1. Install a Blender build through the plugin
2. Refresh the builds list (scrape again) 
3. Verify that the installed build remains visible in the UI with its install status intact
4. No reload should be necessary

The fix ensures that when new builds are scraped, the UI immediately reflects both the new available builds AND maintains the visibility of already installed builds.
