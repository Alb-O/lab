import { BlenderBuildInfo, DownloadProgress, ExtractionProgress, ScrapingStatus, BuildCache, BuildType } from './types';
import { BlenderPluginSettings, DEFAULT_SETTINGS } from './settings';
import { BlenderScraper } from './scraper';
import { BlenderDownloader } from './downloader';
import { BlenderLauncher } from './launcher';
import { BuildFilter } from './buildFilter';
import { Notice } from 'obsidian';
import * as path from 'path';
import * as fs from 'fs';
import { EventEmitter } from 'events';

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
	};
	private cacheFilePath: string;
	private static readonly CACHE_VERSION = '1.0.0';
	private cacheLoadingPromise: Promise<void>;
		// Cache for extracted builds to avoid expensive filesystem operations
	private extractedBuildsCache: Array<{ build: BlenderBuildInfo; extractPath: string; executable?: string }> | null = null;
	private extractedBuildsCacheTime: number = 0;
	private static readonly EXTRACTED_BUILDS_CACHE_TTL = 30000; // 30 seconds

	constructor(vaultPath: string, settings: BlenderPluginSettings = DEFAULT_SETTINGS) {
		super();
		this.vaultPath = vaultPath;
		this.settings = settings;
		this.scraper = new BlenderScraper(settings.minimumBlenderVersion);
		this.downloader = new BlenderDownloader();
		this.launcher = new BlenderLauncher(settings);
		this.buildFilter = new BuildFilter(this);
		this.cacheFilePath = path.join(this.getDownloadsPath(), 'build-cache.json');
		
		this.setupEventListeners();
		this.cacheLoadingPromise = this.loadCachedBuildsAsync();
	}

	/**
	 * Set up event listeners for scraper and downloader
	 */
	private setupEventListeners(): void {
		// Scraper events - we'll ignore detailed status messages and show simple user-friendly messages
		this.scraper.on('status', (status: string) => {
			// Don't update the status with detailed scraper messages during active scraping
			// Let the refreshBuilds method handle the user-facing status messages
			if (!this.scrapingStatus.isActive) {
				this.scrapingStatus.currentTask = status;
				this.emit('scrapingStatus', this.scrapingStatus);
			}
		});

		this.scraper.on('error', (error: string) => {
			this.scrapingStatus.error = error;
			this.scrapingStatus.isActive = false;
			this.emit('scrapingError', error);
			new Notice(`Scraping error: ${error}`);
		});

		// Downloader events
		this.downloader.on('downloadStarted', (build: BlenderBuildInfo, filePath: string) => {
			this.emit('downloadStarted', build, filePath);
			new Notice(`Started downloading ${build.subversion}`);
		});

		this.downloader.on('downloadCompleted', (build: BlenderBuildInfo, filePath: string) => {
			this.emit('downloadCompleted', build, filePath);
			new Notice(`Download completed: ${build.subversion}`);
			// Auto-extract if enabled
			if (this.settings.autoExtract) {
				this.extractBuild(filePath, build)
					.then(() => {
						// Clean up archive after successful auto-extraction if enabled
						if (this.settings.cleanUpAfterExtraction) {
							this.cleanupAfterExtraction(filePath).catch(console.error);
						}
					})
					.catch(console.error);
			}
		});

		this.downloader.on('downloadError', (build: BlenderBuildInfo, error: any) => {
			this.emit('downloadError', build, error);
			new Notice(`Download failed: ${build.subversion} - ${error.message}`);
		});
		this.downloader.on('extractionStarted', (archivePath: string, extractPath: string) => {
			this.emit('extractionStarted', archivePath, extractPath);
			new Notice(`Extracting ${path.basename(archivePath)}...`);
		});

		this.downloader.on('extractionCompleted', (archivePath: string, extractPath: string) => {
			this.emit('extractionCompleted', archivePath, extractPath);
			new Notice(`Extraction completed: ${path.basename(archivePath)}`);
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
	}/**
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
	 * Get cached builds
	 */
	getCachedBuilds(): BlenderBuildInfo[] {
		return [...this.buildCache];
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
			}
			// Clean up archive after successful extraction if enabled
			if (this.settings.cleanUpAfterExtraction) {
				await this.cleanupAfterExtraction(archivePath);
			}
			
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
			await this.cleanupEmptyDirectory(downloadsPath);
		} catch (error) {
			console.warn('Failed to cleanup after extraction:', error);
		}
	}

	/**
	 * Sanitize build name for use as directory name
	 */
	private sanitizeBuildName(build: BlenderBuildInfo): string {
		const name = `${build.subversion}-${build.branch}`;
		return name.replace(/[^a-zA-Z0-9.-]/g, '_');
	}	/**
	 * Find a build by matching patterns in the directory name
	 * This helps detect manually installed builds that don't follow the exact naming convention
	 */
	private findBuildByPattern(dirName: string, expectedBuildType: BuildType): BlenderBuildInfo | undefined {
		// Extract version and branch information from directory name
		// Pattern examples:
		// - blender-4.5.0-alpha+main.6fab30a767df-windows.amd64-release
		// - blender-4.3.2-linux-x64
		// - 4.5.0-main
		
		const builds = this.buildCache.filter(build => this.buildFilter.getBuildType(build) === expectedBuildType);
		
		if (builds.length === 0) {
			return undefined;
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
				...(dirName.includes('main') ? [new RegExp(`${versionPattern}`, 'i')] : [])
			];			
			// Test each pattern
			for (let i = 0; i < patterns.length; i++) {
				const pattern = patterns[i];
				const matches = pattern.test(dirName);
				if (matches) {
					return build;
				}
			}
		}
				return undefined;
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
	 */	
	getExtractedBuilds(): Array<{ build: BlenderBuildInfo; extractPath: string; executable?: string }> {
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
					// Try to match with cached builds based on directory name
					let matchingBuild = this.buildCache.find(build => {
						const expectedDirName = this.sanitizeBuildName(build);
						const matches = dir === expectedDirName && this.buildFilter.getBuildType(build) === buildType;
						return matches;
					});

					// If exact match fails, try pattern matching for manually installed builds
					if (!matchingBuild) {
						matchingBuild = this.findBuildByPattern(dir, buildType);
					}

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
			// Directory might not exist or we don't have permission - ignore
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
	}

	/**
	 * Load cached builds asynchronously without blocking constructor
	 */
	private async loadCachedBuildsAsync(): Promise<void> {
		try {
			await this.loadCachedBuilds();
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

			return builds;
		} catch (error) {
			console.error('Failed to load cached builds:', error);
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
			fs.writeFileSync(this.cacheFilePath, cacheData, 'utf8');
					} catch (error) {
			console.error('Failed to save builds cache:', error);
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
			this.emit('buildsUpdated', []);
		} catch (error) {
			console.error('Failed to clear cache:', error);
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

		try {
			// Delete downloaded archive from segregated downloads path
			const downloadsPath = this.getDownloadsPathForBuild(build);
			const expectedFileName = this.extractFileName(build.link);
			const downloadPath = path.join(downloadsPath, expectedFileName);
			
			if (fs.existsSync(downloadPath)) {
				fs.unlinkSync(downloadPath);
				deletedDownload = true;
							}

			// Delete extracted build from segregated extracts path
			const extractsPath = this.getExtractsPathForBuild(build);
			const expectedDirName = this.sanitizeBuildName(build);
			const extractPath = path.join(extractsPath, expectedDirName);
			
			if (fs.existsSync(extractPath)) {
				await this.deleteDirectory(extractPath);
				deletedExtract = true;
							}			// Clean up empty type directories if needed
			await this.cleanupEmptyDirectory(downloadsPath);
			await this.cleanupEmptyDirectory(extractsPath);

			// Invalidate extracted builds cache if we deleted an extracted build
			if (deletedExtract) {
				this.invalidateExtractedBuildsCache();
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

			return { deletedDownload, deletedExtract };
		} catch (error) {
			this.emit('deletionError', build, error);
			new Notice(`Failed to delete ${build.subversion}: ${error.message}`);
			throw error;
		}
	}
	
	/**
	 * Check if a build is installed (downloaded or extracted)
	 */
	isBuildInstalled(build: BlenderBuildInfo): { downloaded: boolean; extracted: boolean } {
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

		const extractsPath = this.getExtractsPathForBuild(build);
		const expectedDirName = this.sanitizeBuildName(build);
		const extractPath = path.join(extractsPath, expectedDirName);
		
		if (!fs.existsSync(extractPath)) {
			throw new Error('Extracted build directory not found');
		}

		// Use the launcher to launch the build
		await this.launcher.launchBuild(build, extractPath);
	}

	/**
	 * Recursively delete a directory and all its contents
	 */
	private async deleteDirectory(dirPath: string): Promise<void> {
		if (!fs.existsSync(dirPath)) {
			return;
		}

		const entries = fs.readdirSync(dirPath, { withFileTypes: true });
		
		for (const entry of entries) {
			const fullPath = path.join(dirPath, entry.name);
			
			if (entry.isDirectory()) {
				await this.deleteDirectory(fullPath);
			} else {
				fs.unlinkSync(fullPath);
			}
		}
		
		fs.rmdirSync(dirPath);
	}

	/**
	 * Test specific folder detection
	 */
	testSpecificFolderDetection(): void {
		const testDir = "blender-4.5.0-alpha+main.6fab30a767df-windows.amd64-release";
				new Notice(`Testing detection for: ${testDir}`, 5000);
		
		// Try exact match first
		const exactMatch = this.buildCache.find(build => {
			const expectedDirName = this.sanitizeBuildName(build);
			const matches = testDir === expectedDirName && this.buildFilter.getBuildType(build) === BuildType.Daily;
			if (matches) {
							}
			return matches;
		});
		
		if (exactMatch) {
			new Notice(`EXACT MATCH: ${exactMatch.subversion}-${exactMatch.branch}`, 6000);
		} else {
			new Notice("No exact match found, trying pattern match...", 4000);
			
			// Try pattern match
			const patternMatch = this.findBuildByPattern(testDir, BuildType.Daily);
			if (patternMatch) {
				new Notice(`PATTERN MATCH: ${patternMatch.subversion}-${patternMatch.branch}`, 6000);
			} else {
				new Notice("No pattern match found either!", 6000);
			}
		}
	}
}
