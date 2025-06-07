import { BlenderBuildInfo, DownloadProgress, ExtractionProgress, ScrapingStatus, BuildCache, BuildType, InstalledBuildMetadata, InstalledBuildsCache } from './types';
import { BlenderPluginSettings, DEFAULT_SETTINGS } from './settings';
import { BlenderScraper } from './scraper';
import { BlenderDownloader } from './downloader';
import { BlenderLauncher } from './launcher';
import { BuildFilter } from './buildFilter';
import { Notice } from 'obsidian';
import * as path from 'path';
import * as fs from 'fs';
import { EventEmitter } from 'events';
import { 
	debug, 
	info, 
	warn, 
	error,
	registerLoggerClass 
} from './utils/obsidian-logger';

export class FetchBlenderBuilds extends EventEmitter {
	private scraper: BlenderScraper;
	private downloader: BlenderDownloader;
	private launcher: BlenderLauncher;
	private buildFilter: BuildFilter;
	private settings: BlenderPluginSettings;
	private vaultPath: string;
	private buildCache: BlenderBuildInfo[] = [];
	private scrapingStatus: ScrapingStatus = {
		isActive: false,
		currentTask: '',
		progress: 0
	}; private cacheFilePath: string;
	private installedBuildsCache: InstalledBuildMetadata[] = [];
	private installedBuildsCacheFilePath: string;
	private static readonly CACHE_VERSION = '1.0.0';
	private static readonly INSTALLED_BUILDS_CACHE_VERSION = '1.0.0';
	private cacheLoadingPromise: Promise<void>;
	// Cache for extracted builds to avoid expensive filesystem operations
	private extractedBuildsCache: Array<{ build: BlenderBuildInfo; extractPath: string; executable?: string }> | null = null;
	private extractedBuildsCacheTime: number = 0;
	private static readonly EXTRACTED_BUILDS_CACHE_TTL = 30000; // 30 seconds
	
	constructor(vaultPath: string, settings: BlenderPluginSettings = DEFAULT_SETTINGS) {
		super();
		registerLoggerClass(this, 'FetchBlenderBuilds');
		debug(this, 'FetchBlenderBuilds constructor started');
		
		this.vaultPath = vaultPath;
		this.settings = settings;
		this.scraper = new BlenderScraper(settings.minimumBlenderVersion);
		this.downloader = new BlenderDownloader();
		this.launcher = new BlenderLauncher(settings); this.buildFilter = new BuildFilter(this);
		this.cacheFilePath = path.join(this.getDownloadsPath(), 'build-cache.json');
		this.installedBuildsCacheFilePath = path.join(this.getDownloadsPath(), 'installed-builds-cache.json');

		debug(this, `Cache paths configured: ${this.cacheFilePath} and ${this.installedBuildsCacheFilePath}`);

		this.setupEventListeners();
		this.cacheLoadingPromise = this.loadCachedBuildsAsync();
		
		info(this, 'FetchBlenderBuilds constructor completed successfully');
	}

	/**
	 * Set up event listeners for scraper and downloader
	 */
	private setupEventListeners(): void {
		debug(this, 'Setting up event listeners');
		// Scraper events - we'll ignore detailed status messages and show simple user-friendly messages
		this.scraper.on('status', (status: string) => {
			debug(this, `Scraper status: ${status} (active: ${this.scrapingStatus.isActive})`);
			// Don't update the status with detailed scraper messages during active scraping
			// Let the refreshBuilds method handle the user-facing status messages
			if (!this.scrapingStatus.isActive) {
				this.scrapingStatus.currentTask = status;
				this.emit('scrapingStatus', this.scrapingStatus);
			}
		});
		this.scraper.on('error', (errorMsg: string) => {
			error(this, `Scraper error: ${errorMsg}`);
			this.scrapingStatus.error = errorMsg;
			this.scrapingStatus.isActive = false;
			this.emit('scrapingError', errorMsg);
			new Notice(`Scraping error: ${errorMsg}`);
		});

		// Downloader events
		this.downloader.on('downloadStarted', (build: BlenderBuildInfo, filePath: string) => {
			info(this, `Started downloading ${build.subversion} to ${filePath}`);
			this.emit('downloadStarted', build, filePath);
			new Notice(`Started downloading ${build.subversion}`);
		});

		this.downloader.on('downloadCompleted', (build: BlenderBuildInfo, filePath: string) => {
			info(this, `Download completed: ${build.subversion} saved to ${filePath}`);
			this.emit('downloadCompleted', build, filePath);
			new Notice(`Download completed: ${build.subversion}`);
			// Auto-extract if enabled
			if (this.settings.autoExtract) {
				debug(this, `Auto-extraction enabled, starting extraction`);
				this.extractBuild(filePath, build)
					.then(() => {
						info(this, 'Auto-extraction completed successfully');
						// Clean up archive after successful auto-extraction if enabled
						if (this.settings.cleanUpAfterExtraction) {
							debug(this, 'Cleanup after extraction enabled, starting cleanup');
							this.cleanupAfterExtraction(filePath).catch(console.error);
						}
					})
					.catch(console.error);
			}
		});

		this.downloader.on('downloadError', (build: BlenderBuildInfo, errorData: any) => {
			error(this, `Download failed for ${build.subversion}: ${errorData.message || errorData}`);
			this.emit('downloadError', build, errorData);
			new Notice(`Download failed: ${build.subversion} - ${errorData.message}`);
		});
		this.downloader.on('extractionStarted', (archivePath: string, extractPath: string) => {
			info(this, `Started extracting ${path.basename(archivePath)} to ${extractPath}`);
			this.emit('extractionStarted', archivePath, extractPath);
			new Notice(`Extracting ${path.basename(archivePath)}...`);
		});

		this.downloader.on('extractionProgress', (progress: any) => {
			this.emit('extractionProgress', progress);
		});
		
		this.downloader.on('extractionCompleted', (archivePath: string, extractPath: string) => {
			this.emit('extractionCompleted', archivePath, extractPath);
			new Notice(`Extraction completed: ${path.basename(archivePath)}`);
			
			// Invalidate extracted builds cache and refresh UI so the newly extracted build shows up as installed
			this.invalidateExtractedBuildsCache();
			
			// Emit buildsUpdated event to trigger UI refresh
			this.emit('buildsUpdated', this.getCachedBuilds());
		});
		this.downloader.on('extractionError', (archivePath: string, error: any) => {
			this.emit('extractionError', archivePath, error);
			new Notice(`Extraction failed: ${path.basename(archivePath)} - ${error.message}`);
		});

		// Launcher events - forward to our own events
		this.launcher.on('buildLaunched', (build: BlenderBuildInfo, launcherPath: string) => {
			this.emit('buildLaunched', build, launcherPath);
		});

		this.launcher.on('launchError', (build: BlenderBuildInfo, error: any) => {
			this.emit('launchError', build, error);
		});
	}

	/**
	 * Update plugin settings
	 */
	updateSettings(newSettings: Partial<BlenderPluginSettings>): void {
		this.settings = { ...this.settings, ...newSettings };
		this.launcher.updateSettings(this.settings);
		this.emit('settingsUpdated', this.settings);
	}

	/**
	 * Get current settings
	 */
	getSettings(): BlenderPluginSettings {
		return this.settings;
	}

	/**
	 * Get preferred architecture from settings
	 */
	getPreferredArchitecture(): string {
		const settingArch = this.settings.preferredArchitecture || 'auto';

		// If set to auto, detect system architecture
		if (settingArch === 'auto') {
			const arch = process.arch;
			if (arch === 'arm64') {
				return 'arm64';
			} else {
				return 'x64'; // Default to x64 for x64, ia32, etc.
			}
		}

		return settingArch;
	}

	/**
	 * Get builds directory path
	 */
	getBuildsPath(): string {
		return path.join(this.vaultPath, this.settings.libraryFolder);
	}

	/**
	 * Get downloads directory path
	 */
	getDownloadsPath(): string {
		return path.join(this.getBuildsPath(), 'build_archives');
	}
	
	/**
	 * Get builds directory path (where extracted builds are stored)
	 */
	getExtractsPath(): string {
		return path.join(this.getBuildsPath(), 'builds');
	}

	/**
	 * Get build type specific directory path for extracts
	 */
	getExtractsPathForBuild(build: BlenderBuildInfo): string {
		const buildType = this.buildFilter.getBuildType(build);
		return path.join(this.getExtractsPath(), buildType);
	}

	/**
	 * Get build type specific directory path for downloads
	 */
	getDownloadsPathForBuild(build: BlenderBuildInfo): string {
		const buildType = this.buildFilter.getBuildType(build);
		return path.join(this.getDownloadsPath(), buildType);
	}

	/**
	 * Ensure directories exist
	 */
	private ensureDirectories(): void {
		const buildsPath = this.getBuildsPath();
		const downloadsPath = this.getDownloadsPath();
		const extractsPath = this.getExtractsPath();

		[buildsPath, downloadsPath, extractsPath].forEach(dir => {
			if (!fs.existsSync(dir)) {
				fs.mkdirSync(dir, { recursive: true });
			}
		});

		// Create subdirectories for each build type
		const buildTypes = [BuildType.Stable, BuildType.Daily, BuildType.LTS, BuildType.Experimental, BuildType.Patch, BuildType.ReleaseCandidate];
		buildTypes.forEach(buildType => {
			const downloadTypeDir = path.join(downloadsPath, buildType);
			const extractTypeDir = path.join(extractsPath, buildType);

			[downloadTypeDir, extractTypeDir].forEach(dir => {
				if (!fs.existsSync(dir)) {
					fs.mkdirSync(dir, { recursive: true });
				}
			});
		});
	}

	/**
	 * Refresh builds by scraping
	 */
	async refreshBuilds(): Promise<BlenderBuildInfo[]> {
		this.scrapingStatus = {
			isActive: true,
			currentTask: 'Checking for available builds...',
			progress: 0,
			lastChecked: new Date()
		};
		this.emit('scrapingStatus', this.scrapingStatus);
		try {
			const builds = await this.scraper.getAllBuilds();

			this.buildCache = builds;
			this.scrapingStatus.isActive = false;
			this.scrapingStatus.currentTask = 'Check completed';
			this.scrapingStatus.progress = 100;
			this.scrapingStatus.lastChecked = new Date();
			this.emit('scrapingStatus', this.scrapingStatus);
			this.emit('buildsUpdated', builds);

			// Save the builds to cache
			await this.saveCacheBuilds(builds);

			new Notice(`Found ${builds.length} Blender builds.`);

			return builds;
		} catch (error) {
			this.scrapingStatus.isActive = false;
			this.scrapingStatus.error = error instanceof Error ? error.message : 'Unknown error';
			this.emit('scrapingError', this.scrapingStatus.error);
			throw error;
		}
	}

	/**
	 * Get cached builds merged with locally installed builds
	 */
	getCachedBuilds(): BlenderBuildInfo[] {
		const officialBuilds = [...this.buildCache];
		const orphanedBuilds: BlenderBuildInfo[] = [];



		// Convert installed builds metadata to BlenderBuildInfo and find orphaned builds
		for (const metadata of this.installedBuildsCache) {
			// Check if this installed build is already in the official cache
			const exists = officialBuilds.some(build =>
				build.link === metadata.link && build.subversion === metadata.subversion
			);



			if (!exists) {
				// This is an orphaned install - build no longer in official cache
				const orphanedBuild = this.metadataToBlenderBuildInfo(metadata, true);

				orphanedBuilds.push(orphanedBuild);
			}
		}


		// Return merged list with orphaned builds at the end
		return [...officialBuilds, ...orphanedBuilds];
	}

	/**
	 * Download a specific build
	 */
	async downloadBuild(
		build: BlenderBuildInfo,
		onProgress?: (progress: DownloadProgress) => void
	): Promise<string> {
		this.ensureDirectories();
		const downloadsPath = this.getDownloadsPathForBuild(build);

		try {
			const filePath = await this.downloader.downloadBuild(build, downloadsPath, onProgress);
			this.emit('buildDownloaded', build, filePath);
			return filePath;
		} catch (error) {
			this.emit('downloadError', build, error);
			throw error;
		}
	}

	/**
	 * Extract a downloaded build
	 */	async extractBuild(
		archivePath: string,
		build: BlenderBuildInfo,
		onProgress?: (progress: ExtractionProgress) => void
	): Promise<string> {
		this.ensureDirectories();
		const extractsPath = this.getExtractsPathForBuild(build);

		// Emit extraction started event for this specific build
		this.emit('extractionStarted', archivePath);
		// Extract directly to the extracts folder - let the ZIP create its own folder structure
		try {
			const extractedPath = await this.downloader.extractBuild(archivePath, extractsPath);

			// Find the Blender executable
			const executable = this.downloader.findBlenderExecutable(extractedPath);
			if (executable) {
				// Update build info with executable path
				build.customExecutable = executable;
			}			// Clean up archive after successful extraction if enabled
			if (this.settings.cleanUpAfterExtraction) {
				await this.cleanupAfterExtraction(archivePath);
			}

			// Add to installed builds cache
			await this.addInstalledBuild(build, extractedPath, this.settings.cleanUpAfterExtraction ? undefined : archivePath);

			// Invalidate extracted builds cache since we added a new build
			this.invalidateExtractedBuildsCache();

			this.emit('buildExtracted', build, extractedPath, executable);
			return extractedPath;
		} catch (error) {
			this.emit('extractionError', archivePath, error);
			throw error;
		}
	}

	/**
	 * Clean up archive file and empty build_archives folder after extraction
	 */
	async cleanupAfterExtraction(archivePath: string): Promise<void> {
		try {
			const fs = require('fs');
			const path = require('path');

			// Remove the archive file
			fs.unlinkSync(archivePath);
			// Clean up empty build_archives directory
			const downloadsPath = path.dirname(archivePath);
			await this.cleanupEmptyDirectory(downloadsPath);		} catch (error) {
			warn(this, `Failed to cleanup after extraction: ${error}`);
		}
	}
	
	/**
	 * Find a build by matching patterns in the directory name
	 * This helps detect manually installed builds that don't follow the exact naming convention
	 */	private findBuildByPattern(dirName: string, expectedBuildType: BuildType): BlenderBuildInfo | undefined {
		// Extract version and branch information from directory name
		// Pattern examples:
		// - blender-4.5.0-alpha+main.6fab30a767df-windows.amd64-release
		// - blender-4.3.2-linux-x64
		// - 4.5.0-main

		// First, check if we have metadata for this exact extracted path
		const typeDir = path.join(this.getExtractsPath(), expectedBuildType);
		const fullExtractPath = path.join(typeDir, dirName);

		const installedMetadata = this.installedBuildsCache.find(metadata =>
			metadata.extractedPath === fullExtractPath
		);

		const builds = this.buildCache.filter(build => this.buildFilter.getBuildType(build) === expectedBuildType);

		if (installedMetadata) {
			// Check if this build exists in the current online cache
			const existsInCache = builds.some(build =>
				build.link === installedMetadata.link && build.subversion === installedMetadata.subversion
			);

			if (existsInCache) {
				// Find the actual build from cache and return it (not orphaned)
				const cacheMatches = builds.find(build =>
					build.link === installedMetadata.link && build.subversion === installedMetadata.subversion
				);
				if (cacheMatches) {
					return cacheMatches;
				}
			}

			// Use the metadata but mark as orphaned since it's not in current online cache
			return this.metadataToBlenderBuildInfo(installedMetadata, true);
		}

		if (builds.length === 0) {
			// If no builds in cache for this type, create a placeholder orphaned build
			return this.createPlaceholderBuild(dirName, expectedBuildType);
		}
		for (const build of builds) {
			// First, try to match against the filename from the download link
			// Extract filename from URL (e.g., blender-4.5.0-alpha+main.6fab30a767df-windows.amd64-release.zip)
			const linkFilename = build.link.split('/').pop()?.replace(/\.(zip|tar\.(gz|xz|bz2))$/i, '') || '';

			// Direct filename match (most reliable for manually extracted builds)
			if (linkFilename && dirName === linkFilename) {
				return build;
			}

			// Partial filename match (in case the directory name is truncated or modified)
			if (linkFilename && linkFilename.length > 10 && dirName.includes(linkFilename.substring(0, linkFilename.length - 5))) {
				return build;
			}

			// Match by buildHash if present in directory name
			if (build.buildHash && dirName.includes(build.buildHash)) {
				return build;
			}

			// Create patterns for traditional pattern matching as fallback
			const versionPattern = build.subversion.replace(/[+.\-|]/g, '\\$&'); // Escape special regex chars
			const branchPattern = build.branch.replace(/[+.\-]/g, '\\$&');

			// Create a flexible regex that matches version and branch in various formats
			const patterns = [
				// Pattern 1: version+branch (e.g., 4.5.0-alpha+main)
				new RegExp(`${versionPattern}[+]${branchPattern}`, 'i'),
				// Pattern 2: version-branch (e.g., 4.5.0-alpha-main)  
				new RegExp(`${versionPattern}[-]${branchPattern}`, 'i'),
				// Pattern 3: blender-version+branch (e.g., blender-4.5.0-alpha+main)
				new RegExp(`blender[-]${versionPattern}[+]${branchPattern}`, 'i'),
				// Pattern 4: blender-version-branch (e.g., blender-4.5.0-alpha-main)
				new RegExp(`blender[-]${versionPattern}[-]${branchPattern}`, 'i'),
				// Pattern 5: just version in blender filename if branch is main (common case)
				...(build.branch === 'main' ? [new RegExp(`blender[-]${versionPattern}`, 'i')] : []),
				// Pattern 6: blender-version+main for main/daily branch builds (handle main vs daily mismatch)
				new RegExp(`blender[-]${versionPattern}[+]main`, 'i'),
				// Pattern 7: just the version pattern if it's a main branch build with same version
				...(dirName.includes('main') ? [new RegExp(`${versionPattern}`, 'i')] : [])];			// Test each pattern
			for (let i = 0; i < patterns.length; i++) {
				const pattern = patterns[i];
				const matches = pattern.test(dirName);
				if (matches) {
					return build;
				}
			}
		}

		// If no match found in cache, create a placeholder orphaned build
		// This allows us to detect builds that were installed before or when cache was empty
		return this.createPlaceholderBuild(dirName, expectedBuildType);
	}

	/**
	 * Create a placeholder BlenderBuildInfo for an orphaned build that we can't match in cache
	 */	private createPlaceholderBuild(dirName: string, buildType: BuildType): BlenderBuildInfo {
		// Try to extract version info from directory name
		const versionMatch = dirName.match(/(\d+\.\d+(?:\.\d+)?(?:-\w+)?)/);
		const hashMatch = dirName.match(/([a-f0-9]{12,})/);

		const version = versionMatch ? versionMatch[1] : 'unknown';
		const buildHash = hashMatch ? hashMatch[1] : '';
		// Create a synthetic build info for this orphaned build
		return {
			subversion: version,
			branch: buildType === BuildType.Daily ? 'main' : buildType,
			buildHash: buildHash,
			commitTime: new Date(), // Use current date since we don't know the actual build date
			link: `orphaned://${dirName}`, // Use a special orphaned:// scheme to identify these
			isOrphanedInstall: true // Mark as orphaned
		};
	}

	/**
	 * Clean up duplicate orphaned entries in installed builds cache
	 * Only removes orphaned:// entries when there's a corresponding real entry for the same build
	 */
	private cleanupOrphanedDuplicates(): void {
		const originalCount = this.installedBuildsCache.length;
		const orphanedEntries: InstalledBuildMetadata[] = [];
		const realEntries: InstalledBuildMetadata[] = [];



		// Separate orphaned and real entries
		for (const metadata of this.installedBuildsCache) {

			if (metadata.link.startsWith('orphaned://')) {
				orphanedEntries.push(metadata);

			} else {
				realEntries.push(metadata);

			}
		}



		// Find orphaned entries that have corresponding real entries
		const orphanedToRemove: InstalledBuildMetadata[] = [];
		for (const orphaned of orphanedEntries) {


			// Check if there's a real entry with the same extracted path
			const hasRealCounterpart = realEntries.some(real => {
				const pathMatch = real.extractedPath === orphaned.extractedPath;

				// Also check if the orphaned path is a subdirectory of the real path
				// or if they share the same final directory name
				const orphanedDir = path.basename(orphaned.extractedPath || '');
				const realDir = path.basename(real.extractedPath || '');
				const pathSimilarity = orphanedDir === realDir ||
					orphaned.extractedPath?.includes(real.extractedPath || '') ||
					real.extractedPath?.includes(orphaned.extractedPath || '');

				const metaMatch = real.subversion === orphaned.subversion &&
					real.buildHash === orphaned.buildHash &&
					real.commitTime === orphaned.commitTime;

				// For more lenient matching, also compare by subversion and build hash
				const buildMatch = real.subversion === orphaned.subversion &&
					real.buildHash === orphaned.buildHash;







				return pathMatch || pathSimilarity || metaMatch || buildMatch;
			});

			if (hasRealCounterpart) {

				orphanedToRemove.push(orphaned);
			} else {

			}
		}

		// Remove the duplicates
		if (orphanedToRemove.length > 0) {
			this.installedBuildsCache = this.installedBuildsCache.filter(metadata =>
				!orphanedToRemove.includes(metadata)
			);




			// Save the cleaned cache
			this.saveInstalledBuildsCache();
		} else {

		}
	}

	/**
	 * Debug method to show what builds are in cache
	 */
	debugShowCacheContents(): void {

		// Group builds by type
		const buildsByType: Record<string, BlenderBuildInfo[]> = {};
		for (const build of this.buildCache) {
			const buildType = this.buildFilter.getBuildType(build);
			if (!buildsByType[buildType]) {
				buildsByType[buildType] = [];
			}
			buildsByType[buildType].push(build);
		}

		// Show summary by type
		for (const [type, builds] of Object.entries(buildsByType)) {

			// Show first few builds of each type
			builds.slice(0, 2).forEach(build => {
			});
		}

		// Look specifically for builds that might match your manual installation
		const potentialMatches = this.buildCache.filter(build =>
			build.subversion.includes('4.5.0') &&
			build.branch.toLowerCase().includes('main')
		);

		new Notice(`Found ${potentialMatches.length} potential matches for 4.5.0-main builds`, 6000);

		potentialMatches.slice(0, 5).forEach(build => {
			new Notice(`Match candidate: ${build.subversion}-${build.branch}`, 5000);
		});
	}

	/**
	 * Get downloaded builds
	 */
	getDownloadedBuilds(): Array<{ build: BlenderBuildInfo; filePath: string }> {
		const downloadsPath = this.getDownloadsPath();
		if (!fs.existsSync(downloadsPath)) {
			return [];
		}

		const downloadedBuilds: Array<{ build: BlenderBuildInfo; filePath: string }> = [];

		// Check each build type subdirectory
		const buildTypes = [BuildType.Stable, BuildType.Daily, BuildType.LTS, BuildType.Experimental, BuildType.Patch, BuildType.ReleaseCandidate];

		for (const buildType of buildTypes) {
			const typeDir = path.join(downloadsPath, buildType);
			if (!fs.existsSync(typeDir)) continue;

			const files = fs.readdirSync(typeDir);

			for (const file of files) {
				const filePath = path.join(typeDir, file);
				const stats = fs.statSync(filePath);

				if (stats.isFile()) {
					// Try to match with cached builds
					const matchingBuild = this.buildCache.find(build => {
						const expectedFileName = this.extractFileName(build.link);
						return file === expectedFileName && this.buildFilter.getBuildType(build) === buildType;
					});

					if (matchingBuild) {
						downloadedBuilds.push({ build: matchingBuild, filePath });
					}
				}
			}
		}

		return downloadedBuilds;
	}
	/**
	 * Invalidate the extracted builds cache
	 */
	private invalidateExtractedBuildsCache(): void {
		this.extractedBuildsCache = null;
		this.extractedBuildsCacheTime = 0;
	}

	/**
	 * Get extracted builds
	 */	getExtractedBuilds(): Array<{ build: BlenderBuildInfo; extractPath: string; executable?: string }> {
		const now = Date.now();

		// Return cached results if they're still valid
		if (this.extractedBuildsCache && (now - this.extractedBuildsCacheTime) < FetchBlenderBuilds.EXTRACTED_BUILDS_CACHE_TTL) {
			return this.extractedBuildsCache;
		}

		const extractsPath = this.getExtractsPath();

		if (!fs.existsSync(extractsPath)) {
			this.extractedBuildsCache = [];
			this.extractedBuildsCacheTime = now;
			return [];
		}

		const extractedBuilds: Array<{ build: BlenderBuildInfo; extractPath: string; executable?: string }> = [];

		// Check each build type subdirectory
		const buildTypes = [BuildType.Stable, BuildType.Daily, BuildType.LTS, BuildType.Experimental, BuildType.Patch, BuildType.ReleaseCandidate];

		for (const buildType of buildTypes) {
			const typeDir = path.join(extractsPath, buildType);

			if (!fs.existsSync(typeDir)) {
				continue;
			}

			const dirs = fs.readdirSync(typeDir);

			for (const dir of dirs) {
				const extractPath = path.join(typeDir, dir);
				const stats = fs.statSync(extractPath);

				if (stats.isDirectory()) {
					// Try to match with cached builds using pattern matching
					let matchingBuild = this.findBuildByPattern(dir, buildType);

					if (matchingBuild) {
						const executable = this.downloader.findBlenderExecutable(extractPath);
						extractedBuilds.push({
							build: matchingBuild,
							extractPath,
							executable: executable || undefined
						});
					}
				}
			}
		}

		// Cache the results
		this.extractedBuildsCache = extractedBuilds;
		this.extractedBuildsCacheTime = now;

		return extractedBuilds;
	}	/**
	 * Force refresh of extracted builds detection (for debugging)
	 */
	async forceRefreshExtractedBuilds(): Promise<Array<{ build: BlenderBuildInfo; extractPath: string; executable?: string }>> {

		// Force cache invalidation and refresh
		this.invalidateExtractedBuildsCache();
		const extractedBuilds = this.getExtractedBuilds();


		return extractedBuilds;
	}
	/**
	 * Clean up empty directories
	 */
	private async cleanupEmptyDirectory(dirPath: string): Promise<void> {
		try {
			const fs = require('fs');
			const files = fs.readdirSync(dirPath);
			if (files.length === 0) {
				fs.rmdirSync(dirPath);
			}
		} catch (error) {
			// Directory might not exist, we don't have permission, or it's in use - ignore
			// This is a cleanup operation and shouldn't fail the main operation
		}
	}

	/**
	 * Extract file name from URL
	 */
	private extractFileName(url: string): string {
		const urlParts = url.split('/');
		let fileName = urlParts[urlParts.length - 1] || 'blender-build.zip';

		// Remove query parameters
		const queryIndex = fileName.indexOf('?');
		if (queryIndex !== -1) {
			fileName = fileName.substring(0, queryIndex);
		}

		return fileName;
	}

	/**
	 * Check for new builds
	 */
	async checkForNewBuilds(): Promise<BlenderBuildInfo[]> {
		const lastCheck = this.scrapingStatus.lastChecked;
		const newBuilds = await this.scraper.checkForNewBuilds(lastCheck);

		if (newBuilds.length > 0) {
			new Notice(`Found ${newBuilds.length} new Blender builds.`);
		}

		return newBuilds;
	}

	/**
	 * Get latest build for each branch
	 */
	getLatestBuilds(): Record<string, BlenderBuildInfo> {
		const latestBuilds: Record<string, BlenderBuildInfo> = {};

		for (const build of this.buildCache) {
			const currentLatest = latestBuilds[build.branch];
			if (!currentLatest || build.commitTime > currentLatest.commitTime) {
				latestBuilds[build.branch] = build;
			}
		}

		return latestBuilds;
	}

	/**
	 * Get scraping status
	 */
	getScrapingStatus(): ScrapingStatus {
		return { ...this.scrapingStatus };
	}

	/**
	 * Cancel all active downloads
	 */
	cancelAllDownloads(): number {
		const activeDownloads = this.downloader.getActiveDownloads();
		let cancelledCount = 0;

		for (const downloadId of activeDownloads) {
			if (this.downloader.cancelDownload(downloadId)) {
				cancelledCount++;
			}
		}

		return cancelledCount;
	}

	/**
	 * Open builds directory in system file explorer
	 */
	openBuildsDirectory(): void {
		const buildsPath = this.getBuildsPath();

		// Create directory if it doesn't exist
		if (!fs.existsSync(buildsPath)) {
			fs.mkdirSync(buildsPath, { recursive: true });
		}

		// Open directory based on platform
		const { exec } = require('child_process');
		const platform = process.platform;

		let command: string;
		if (platform === 'win32') {
			command = `explorer "${buildsPath}"`;
		} else if (platform === 'darwin') {
			command = `open "${buildsPath}"`;
		} else {
			command = `xdg-open "${buildsPath}"`;
		}

		exec(command, (error) => {
			if (error) {
				new Notice(`Failed to open directory: ${error.message}`);
			}
		});
	}	/**
	 * Load cached builds asynchronously without blocking constructor
	 */	private async loadCachedBuildsAsync(): Promise<void> {
		try {
			await this.loadCachedBuilds();
			await this.loadInstalledBuildsCache();

			// Run migration for existing builds (simple approach)  
			await this.scanAndMigrateExistingBuilds();
		} catch (error) {
		}
	}

	/**
	 * Load cached builds from disk
	 */
	private async loadCachedBuilds(): Promise<BlenderBuildInfo[]> {
		try {
			if (!fs.existsSync(this.cacheFilePath)) {
				return [];
			}

			const cacheData = fs.readFileSync(this.cacheFilePath, 'utf8');
			const cache: BuildCache = JSON.parse(cacheData);

			// Validate cache version
			if (cache.version !== FetchBlenderBuilds.CACHE_VERSION) {
				return [];
			}

			// Parse dates back from ISO strings
			const builds = cache.builds.map(build => ({
				...build,
				commitTime: new Date(build.commitTime)
			}));

			this.buildCache = builds;

			// Update scraping status with cache info
			this.scrapingStatus.lastChecked = new Date(cache.lastUpdated);
			this.emit('scrapingStatus', this.scrapingStatus);

			// Emit cached builds
			if (builds.length > 0) {
				this.emit('buildsUpdated', builds);
			}

			return builds;		} catch (error) {
			error(this, `Failed to load cached builds: ${error}`);
			// Remove invalid cache file
			if (fs.existsSync(this.cacheFilePath)) {
				fs.unlinkSync(this.cacheFilePath);
			}
			return [];
		}
	}

	/**
	 * Save builds to cache
	 */
	private async saveCacheBuilds(builds: BlenderBuildInfo[]): Promise<void> {
		try {
			this.ensureDirectories();

			const cache: BuildCache = {
				builds: builds,
				lastUpdated: new Date().toISOString(),
				version: FetchBlenderBuilds.CACHE_VERSION
			};

			const cacheData = JSON.stringify(cache, null, 2);
			fs.writeFileSync(this.cacheFilePath, cacheData, 'utf8');		} catch (error) {
			error(this, `Failed to save builds cache: ${error}`);
		}
	}

	/**
	 * Clear cached builds
	 */
	clearCache(): void {
		try {
			if (fs.existsSync(this.cacheFilePath)) {
				fs.unlinkSync(this.cacheFilePath);
			}
			this.buildCache = [];
			this.emit('buildsUpdated', []);		} catch (error) {
			error(this, `Failed to clear cache: ${error}`);
		}
	}

	/**
	 * Wait for cached builds to be loaded
	 */
	async waitForCacheLoading(): Promise<void> {
		await this.cacheLoadingPromise;
	}

	/**
	 * Check if cached builds are available
	 */
	hasCachedBuilds(): boolean {
		return this.buildCache.length > 0;
	}

	/**
	 * Get cache age in milliseconds
	 */
	getCacheAge(): number | null {
		if (!this.scrapingStatus.lastChecked) {
			return null;
		}
		return Date.now() - this.scrapingStatus.lastChecked.getTime();
	}

	/**
	 * Delete a build completely (both downloaded archive and extracted files)
	 */
	async deleteBuild(build: BlenderBuildInfo): Promise<{ deletedDownload: boolean; deletedExtract: boolean }> {
		let deletedDownload = false;
		let deletedExtract = false;
		try {			// Handle orphaned builds differently  
			if (build.isOrphanedInstall) {
				// Delete archived file if it exists
				if (build.archivePath && fs.existsSync(build.archivePath)) {
					try {
						fs.unlinkSync(build.archivePath);
						deletedDownload = true;
					} catch (error) {
						if (error.code === 'EPERM' || error.code === 'EBUSY') {
							throw new Error(`Cannot delete archive file - it may be in use by Blender or another application. Please close Blender and try again.`);
						}
						throw error;
					}
				}

				// Delete extracted build if it exists
				if (build.extractedPath && fs.existsSync(build.extractedPath)) {
					await this.deleteDirectory(build.extractedPath);
					deletedExtract = true;
					// Clean up empty type directory if needed
					const extractsPath = path.dirname(build.extractedPath);
					await this.cleanupEmptyDirectory(extractsPath);
				}
			} else {
				// Delete downloaded archive from segregated downloads path
				const downloadsPath = this.getDownloadsPathForBuild(build);
				const expectedFileName = this.extractFileName(build.link);
				const downloadPath = path.join(downloadsPath, expectedFileName);

				if (fs.existsSync(downloadPath)) {
					try {
						fs.unlinkSync(downloadPath);
						deletedDownload = true;
					} catch (error) {
						if (error.code === 'EPERM' || error.code === 'EBUSY') {
							throw new Error(`Cannot delete archive file - it may be in use by Blender or another application. Please close Blender and try again.`);
						}
						throw error;
					}
				}

				// Delete extracted build - find the actual directory using pattern matching
				const extractedBuilds = this.getExtractedBuilds();
				const extractedBuild = extractedBuilds.find(eb => eb.build === build);

				if (extractedBuild && fs.existsSync(extractedBuild.extractPath)) {
					await this.deleteDirectory(extractedBuild.extractPath);
					deletedExtract = true;
					// Clean up empty type directory if needed
					const extractsPath = path.dirname(extractedBuild.extractPath);
					await this.cleanupEmptyDirectory(extractsPath);
				}

				// Clean up empty downloads directory if needed
				await this.cleanupEmptyDirectory(downloadsPath);
			}// Invalidate extracted builds cache if we deleted an extracted build
			if (deletedExtract) {
				this.invalidateExtractedBuildsCache();
			}

			// Remove from installed builds cache if anything was deleted
			if (deletedDownload || deletedExtract) {
				await this.removeInstalledBuild(build);
			}

			// Emit deletion event
			this.emit('buildDeleted', build, { deletedDownload, deletedExtract });
			const deletedItems: string[] = [];
			if (deletedDownload) deletedItems.push('download');
			if (deletedExtract) deletedItems.push('extracted files');

			if (deletedItems.length > 0) {
				new Notice(`Deleted ${build.subversion}: ${deletedItems.join(' and ')}`);
			} else {
				new Notice(`No installed files found for ${build.subversion}`);
			}

			return { deletedDownload, deletedExtract };		} catch (error) {
			this.emit('deletionError', build, error);
			
			// Provide specific error message for permission issues
			if (error.message.includes('Please close Blender')) {
				throw new Error(`Cannot delete ${build.subversion} - ${error.message}`);
			} else if (error.code === 'EPERM' || error.code === 'EBUSY') {
				throw new Error(`Cannot delete ${build.subversion} - files may be in use by Blender or another application. Please close Blender and try again.`);
			} else {
				throw new Error(`Failed to delete ${build.subversion}: ${error.message}`);
			}
		}
	}

	/**
 	* Check if a build is installed (downloaded or extracted)
 	*/
	isBuildInstalled(build: BlenderBuildInfo): { downloaded: boolean; extracted: boolean } {
		// Handle orphaned builds differently
		if (build.isOrphanedInstall) {
			const downloaded = build.archivePath ? fs.existsSync(build.archivePath) : false;
			const extracted = build.extractedPath ? fs.existsSync(build.extractedPath) : false;
			return { downloaded, extracted };
		}

		// Handle regular builds
		const downloadsPath = this.getDownloadsPathForBuild(build);
		const expectedFileName = this.extractFileName(build.link);
		const downloadPath = path.join(downloadsPath, expectedFileName);
		const downloaded = fs.existsSync(downloadPath);

		// Use cached extraction results for performance
		const extractedBuilds = this.getExtractedBuilds();
		const extracted = extractedBuilds.some(extracted => extracted.build === build);

		return { downloaded, extracted };
	}

	/**
	 * Launch a Blender build
	 */
	async launchBuild(build: BlenderBuildInfo): Promise<void> {
		const installStatus = this.isBuildInstalled(build);
		if (!installStatus.extracted) {
			throw new Error('Build must be extracted to launch');
		}

		let extractPath: string;

		// Handle orphaned builds differently
		if (build.isOrphanedInstall && build.extractedPath) {
			extractPath = build.extractedPath;
			if (!fs.existsSync(extractPath)) {
				throw new Error('Extracted build directory not found on filesystem');
			}
		} else {
			// Get the actual extracted build path from detected builds instead of reconstructing it
			const extractedBuilds = this.getExtractedBuilds();
			const extractedBuild = extractedBuilds.find(extracted => extracted.build === build);

			if (!extractedBuild) {
				throw new Error('Extracted build directory not found');
			}

			extractPath = extractedBuild.extractPath;

			if (!fs.existsSync(extractPath)) {
				throw new Error('Extracted build directory not found on filesystem');
			}
		}

		// Use the launcher to launch the build
		await this.launcher.launchBuild(build, extractPath);

		// Update last launched time in installed builds cache
		await this.updateBuildLastLaunched(build);
	}

	/**
	 * Create a symlink named 'bl_symlink' in the root builds directory pointing to the extracted build
	 */
	async symlinkBuild(build: BlenderBuildInfo): Promise<void> {
		let buildPath: string;

		// Handle orphaned builds differently
		if (build.isOrphanedInstall && build.extractedPath) {
			buildPath = build.extractedPath;
			if (!fs.existsSync(buildPath)) {
				throw new Error('Extracted build directory not found on filesystem');
			}
		} else {
			// Get extracted builds once and reuse the results
			const extractedBuilds = this.getExtractedBuilds();
			const extractedBuild = extractedBuilds.find(extracted => extracted.build === build);

			if (!extractedBuild) {
				throw new Error('Build must be extracted to create symlink');
			}

			buildPath = extractedBuild.extractPath;

			if (!fs.existsSync(buildPath)) {
				throw new Error('Extracted build directory not found on filesystem');
			}
		}

		const buildsRootPath = this.getBuildsPath(); const symlinkPath = path.join(buildsRootPath, 'bl_symlink');

		// Remove existing symlink if it exists (including broken symlinks)
		try {
			const stats = fs.lstatSync(symlinkPath);
			// If lstatSync succeeds, something exists at this path
			if (stats.isSymbolicLink() || (process.platform === 'win32' && stats.isDirectory())) {
				// On Windows, junctions appear as directories but can be safely unlinked
				// On other platforms, check for symbolic links
				if (process.platform === 'win32') {
					// Use rmSync for Windows junctions as they can be stubborn
					fs.rmSync(symlinkPath, { recursive: false, force: true });
				} else {
					fs.unlinkSync(symlinkPath);
				}
			} else {
				throw new Error('bl_symlink exists but is not a symlink - cannot replace');
			}
		} catch (error: any) {
			// If lstatSync throws ENOENT, the path doesn't exist, which is fine
			if (error.code !== 'ENOENT') {
				throw new Error(`Failed to remove existing bl_symlink: ${error.message}`);
			}
		}
		try {
			// Create the symlink - use platform-appropriate type
			const symlinkType = process.platform === 'win32' ? 'junction' : 'dir';
			fs.symlinkSync(buildPath, symlinkPath, symlinkType);
			this.emit('buildSymlinked', build, symlinkPath);
			new Notice(`Created symlink: bl_symlink -> ${build.subversion}`);
		} catch (error) {
			throw new Error(`Failed to create symlink: ${error.message}`);
		}
	}
	/**
	 * Recursively delete a directory and all its contents
	 */
	private async deleteDirectory(dirPath: string): Promise<void> {
		if (!fs.existsSync(dirPath)) {
			return;
		}

		try {
			const entries = fs.readdirSync(dirPath, { withFileTypes: true });

			for (const entry of entries) {
				const fullPath = path.join(dirPath, entry.name);

				if (entry.isDirectory()) {
					await this.deleteDirectory(fullPath);
				} else {
					try {
						fs.unlinkSync(fullPath);
					} catch (error) {
						// Handle permission errors specifically
						if (error.code === 'EPERM' || error.code === 'EBUSY') {
							throw new Error(`Cannot delete file "${fullPath}" - it may be in use by Blender or another application. Please close Blender and try again.`);
						}
						throw error;
					}
				}
			}

			try {
				fs.rmdirSync(dirPath);
			} catch (error) {
				// Handle permission errors for directories
				if (error.code === 'EPERM' || error.code === 'EBUSY') {
					throw new Error(`Cannot delete directory "${dirPath}" - it may contain files in use by Blender or another application. Please close Blender and try again.`);
				}
				throw error;
			}
		} catch (error) {
			// Re-throw with context if it's not already our custom error
			if (!error.message.includes('Please close Blender')) {
				throw new Error(`Failed to delete directory "${dirPath}": ${error.message}`);
			}
			throw error;
		}
	}

	/**
	 * Load installed builds metadata from cache
	 */
	private async loadInstalledBuildsCache(): Promise<InstalledBuildMetadata[]> {
		try {
			if (!fs.existsSync(this.installedBuildsCacheFilePath)) {
				return [];
			}

			const cacheData = fs.readFileSync(this.installedBuildsCacheFilePath, 'utf8');
			const cache: InstalledBuildsCache = JSON.parse(cacheData);

			// Validate cache version
			if (cache.version !== FetchBlenderBuilds.INSTALLED_BUILDS_CACHE_VERSION) {
				return [];
			}

			this.installedBuildsCache = cache.builds;

			// Clean up any orphaned duplicates after loading
			this.cleanupOrphanedDuplicates();

			return this.installedBuildsCache;		} catch (error) {
			error(this, `Failed to load installed builds cache: ${error}`);
			// Remove invalid cache file
			if (fs.existsSync(this.installedBuildsCacheFilePath)) {
				fs.unlinkSync(this.installedBuildsCacheFilePath);
			}
			return [];
		}
	}

	/**
	 * Save installed builds metadata to cache
	 */
	private async saveInstalledBuildsCache(): Promise<void> {
		try {
			this.ensureDirectories();

			const cache: InstalledBuildsCache = {
				builds: this.installedBuildsCache,
				lastUpdated: new Date().toISOString(),
				version: FetchBlenderBuilds.INSTALLED_BUILDS_CACHE_VERSION
			};

			const cacheData = JSON.stringify(cache, null, 2);
			fs.writeFileSync(this.installedBuildsCacheFilePath, cacheData, 'utf8');		} catch (error) {
			error(this, `Failed to save installed builds cache: ${error}`);
		}
	}

	/**
	 * Add or update an installed build in the metadata cache
	 */
	async addInstalledBuild(build: BlenderBuildInfo, extractedPath?: string, archivePath?: string): Promise<void> {
		const buildType = this.buildFilter.getBuildType(build);

		// Check if build already exists in cache
		const existingIndex = this.installedBuildsCache.findIndex(
			cached => cached.link === build.link && cached.subversion === build.subversion
		);

		const metadata: InstalledBuildMetadata = {
			link: build.link,
			subversion: build.subversion,
			buildHash: build.buildHash,
			commitTime: build.commitTime.toISOString(),
			branch: build.branch,
			extractedPath,
			archivePath,
			customExecutable: build.customExecutable,
			installedAt: existingIndex >= 0 ? this.installedBuildsCache[existingIndex].installedAt : new Date().toISOString(),
			buildType
		};

		if (existingIndex >= 0) {
			// Update existing
			this.installedBuildsCache[existingIndex] = metadata;
		} else {
			// Add new
			this.installedBuildsCache.push(metadata);
		}

		await this.saveInstalledBuildsCache();
	}

	/**
	 * Remove an installed build from the metadata cache
	 */
	async removeInstalledBuild(build: BlenderBuildInfo): Promise<void> {
		const index = this.installedBuildsCache.findIndex(
			cached => cached.link === build.link && cached.subversion === build.subversion
		);

		if (index >= 0) {
			this.installedBuildsCache.splice(index, 1);
			await this.saveInstalledBuildsCache();
		}
	}

	/**
	 * Update last launched time for an installed build
	 */
	async updateBuildLastLaunched(build: BlenderBuildInfo): Promise<void> {
		const index = this.installedBuildsCache.findIndex(
			cached => cached.link === build.link && cached.subversion === build.subversion
		);

		if (index >= 0) {
			this.installedBuildsCache[index].lastLaunched = new Date().toISOString();
			await this.saveInstalledBuildsCache();
		}
	}

	/**
	 * Convert installed build metadata to BlenderBuildInfo
	 */	private metadataToBlenderBuildInfo(metadata: InstalledBuildMetadata, isOrphaned: boolean = true): BlenderBuildInfo {
		return {
			link: metadata.link,
			subversion: metadata.subversion,
			buildHash: metadata.buildHash,
			commitTime: new Date(metadata.commitTime),
			branch: metadata.branch,
			customExecutable: metadata.customExecutable,
			isOrphanedInstall: isOrphaned, // Mark as orphaned only if specified
			extractedPath: metadata.extractedPath,
			archivePath: metadata.archivePath
		};
	}

	/**
	 * Clear installed builds cache
	 */
	clearInstalledBuildsCache(): void {
		this.installedBuildsCache = [];
		if (fs.existsSync(this.installedBuildsCacheFilePath)) {
			fs.unlinkSync(this.installedBuildsCacheFilePath);
		}
	}

	/**
	 * Get count of installed builds that are orphaned (not in official cache)
	 */
	getOrphanedBuildsCount(): number {
		const officialBuilds = this.buildCache;
		let orphanedCount = 0;

		for (const metadata of this.installedBuildsCache) {
			// Check if this installed build is not in the official cache
			const exists = officialBuilds.some(build =>
				build.link === metadata.link && build.subversion === metadata.subversion
			);

			if (!exists) {
				orphanedCount++;
			}
		}

		return orphanedCount;
	}

	/**
	 * Get all installed builds metadata (both current and orphaned)
	 */
	getInstalledBuildsMetadata(): InstalledBuildMetadata[] {
		return [...this.installedBuildsCache];
	}
	/**
	 * Scan for existing installed builds and add them to metadata cache (simplified migration)
	 * This detects builds that were installed before the metadata system
	 */	async scanAndMigrateExistingBuilds(): Promise<void> {
		try {
			// Get all currently extracted builds using the existing detection system
			const extractedBuilds = this.getExtractedBuilds();
			let migratedCount = 0;

			for (const extractedBuild of extractedBuilds) {
				const build = extractedBuild.build;

				// Skip orphaned builds - they shouldn't be migrated
				if (build.isOrphanedInstall || build.link.startsWith('orphaned://')) {

					continue;
				}

				// Check if this build is already in our installed builds cache
				const existsInCache = this.installedBuildsCache.some(
					cached => cached.link === build.link && cached.subversion === build.subversion
				);

				if (!existsInCache) {
					// This is an existing build that needs to be migrated

					await this.addInstalledBuild(build, extractedBuild.extractPath);
					migratedCount++;
				}
			}

			if (migratedCount > 0) {

				new Notice(`Found and added ${migratedCount} existing build(s) to the tracking system.`);

				// Run cleanup again after migration to remove any orphaned duplicates that might have been created
				this.cleanupOrphanedDuplicates();

				// Emit event to update UI
				this.emit('buildsUpdated', this.getCachedBuilds());
			}		} catch (error) {
			error(this, `Failed to migrate existing builds: ${error}`);
		}
	}
}