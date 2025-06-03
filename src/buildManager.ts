import { BlenderBuildInfo, DownloadProgress, ExtractionProgress, ScrapingStatus, BuildCache } from './types';
import { BlenderPluginSettings, DEFAULT_SETTINGS } from './settings';
import { BlenderScraper } from './scraper';
import { BlenderDownloader } from './downloader';
import { Notice } from 'obsidian';
import * as path from 'path';
import * as fs from 'fs';
import { EventEmitter } from 'events';

export class FetchBlenderBuilds extends EventEmitter {
	private scraper: BlenderScraper;
	private downloader: BlenderDownloader;
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

	constructor(vaultPath: string, settings: BlenderPluginSettings = DEFAULT_SETTINGS) {
		super();
		this.vaultPath = vaultPath;
		this.settings = settings;
		this.scraper = new BlenderScraper(settings.minimumBlenderVersion);
		this.downloader = new BlenderDownloader();
		this.cacheFilePath = path.join(this.getBuildsPath(), 'build-cache.json');
		
		this.setupEventListeners();
		this.cacheLoadingPromise = this.loadCachedBuildsAsync();
	}

	/**
	 * Set up event listeners for scraper and downloader
	 */	private setupEventListeners(): void {
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
			if (this.settings.showNotifications) {
				new Notice(`Scraping error: ${error}`);
			}
		});

		// Downloader events
		this.downloader.on('downloadStarted', (build: BlenderBuildInfo, filePath: string) => {
			this.emit('downloadStarted', build, filePath);
			if (this.settings.showNotifications) {
				new Notice(`Started downloading ${build.subversion}`);
			}
		});

		this.downloader.on('downloadCompleted', (build: BlenderBuildInfo, filePath: string) => {
			this.emit('downloadCompleted', build, filePath);
			if (this.settings.showNotifications) {
				new Notice(`Download completed: ${build.subversion}`);
			}
			
			// Auto-extract if enabled
			if (this.settings.autoExtract) {
				this.extractBuild(filePath, build).catch(console.error);
			}
		});

		this.downloader.on('downloadError', (build: BlenderBuildInfo, error: any) => {
			this.emit('downloadError', build, error);
			if (this.settings.showNotifications) {
				new Notice(`Download failed: ${build.subversion} - ${error.message}`);
			}
		});

		this.downloader.on('extractionStarted', (archivePath: string, extractPath: string) => {
			this.emit('extractionStarted', archivePath, extractPath);
		});

		this.downloader.on('extractionCompleted', (archivePath: string, extractPath: string) => {
			this.emit('extractionCompleted', archivePath, extractPath);
			if (this.settings.showNotifications) {
				new Notice(`Extraction completed: ${path.basename(archivePath)}`);
			}
			
			// Cleanup archive if enabled
			if (this.settings.cleanupArchives) {
				try {
					fs.unlinkSync(archivePath);
				} catch (error) {
					console.warn('Failed to cleanup archive:', error);
				}
			}
		});

		this.downloader.on('extractionError', (archivePath: string, error: any) => {
			this.emit('extractionError', archivePath, error);
			if (this.settings.showNotifications) {
				new Notice(`Extraction failed: ${path.basename(archivePath)} - ${error.message}`);
			}
		});
	}

	/**
	 * Update plugin settings
	 */
	updateSettings(newSettings: Partial<BlenderPluginSettings>): void {
		this.settings = { ...this.settings, ...newSettings };
		this.scraper = new BlenderScraper(this.settings.minimumBlenderVersion);
		this.emit('settingsUpdated', this.settings);
	}

	/**
	 * Get current settings
	 */
	getSettings(): BlenderPluginSettings {
		return this.settings;
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
		return path.join(this.getBuildsPath(), 'downloads');
	}	/**
	 * Get builds directory path (where extracted builds are stored)
	 */
	getExtractsPath(): string {
		return path.join(this.getBuildsPath(), 'builds');
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
			const builds = await this.scraper.getAllBuilds(
				this.settings.scrapeStableBuilds,
				this.settings.scrapeAutomatedBuilds
			);

			this.buildCache = builds;
			this.scrapingStatus.isActive = false;
			this.scrapingStatus.currentTask = 'Check completed';
			this.scrapingStatus.progress = 100;
			this.scrapingStatus.lastChecked = new Date();
			this.emit('scrapingStatus', this.scrapingStatus);
			this.emit('buildsUpdated', builds);

			// Save the builds to cache
			await this.saveCacheBuilds(builds);

			if (this.settings.showNotifications) {
				new Notice(`Found ${builds.length} Blender builds`);
			}

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
		const downloadsPath = this.getDownloadsPath();
		
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
		const extractsPath = this.getExtractsPath();
		
		// Extract directly to the extracts folder - let the ZIP create its own folder structure
		try {
			const extractedPath = await this.downloader.extractBuild(archivePath, extractsPath);
			
			// Find the Blender executable
			const executable = this.downloader.findBlenderExecutable(extractedPath);
			if (executable) {
				// Update build info with executable path
				build.customExecutable = executable;
			}
			
			this.emit('buildExtracted', build, extractedPath, executable);
			return extractedPath;
		} catch (error) {
			this.emit('extractionError', archivePath, error);
			throw error;
		}
	}

	/**
	 * Clean up archive file and empty downloads folder after extraction
	 */
	async cleanupAfterExtraction(archivePath: string): Promise<void> {
		try {
			const fs = require('fs');
			const path = require('path');
			
			// Remove the archive file
			fs.unlinkSync(archivePath);
			console.log(`Cleaned up archive: ${archivePath}`);
			
			// Clean up empty downloads directory
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
	}

	/**
	 * Get downloaded builds
	 */
	getDownloadedBuilds(): Array<{ build: BlenderBuildInfo; filePath: string }> {
		const downloadsPath = this.getDownloadsPath();
		if (!fs.existsSync(downloadsPath)) {
			return [];
		}

		const files = fs.readdirSync(downloadsPath);
		const downloadedBuilds: Array<{ build: BlenderBuildInfo; filePath: string }> = [];

		for (const file of files) {
			const filePath = path.join(downloadsPath, file);
			const stats = fs.statSync(filePath);
			
			if (stats.isFile()) {
				// Try to match with cached builds
				const matchingBuild = this.buildCache.find(build => {
					const expectedFileName = this.extractFileName(build.link);
					return file === expectedFileName;
				});

				if (matchingBuild) {
					downloadedBuilds.push({ build: matchingBuild, filePath });
				}
			}
		}

		return downloadedBuilds;
	}

	/**
	 * Get extracted builds
	 */
	getExtractedBuilds(): Array<{ build: BlenderBuildInfo; extractPath: string; executable?: string }> {
		const extractsPath = this.getExtractsPath();
		if (!fs.existsSync(extractsPath)) {
			return [];
		}

		const dirs = fs.readdirSync(extractsPath);
		const extractedBuilds: Array<{ build: BlenderBuildInfo; extractPath: string; executable?: string }> = [];

		for (const dir of dirs) {
			const extractPath = path.join(extractsPath, dir);
			const stats = fs.statSync(extractPath);
			
			if (stats.isDirectory()) {
				// Try to match with cached builds based on directory name
				const matchingBuild = this.buildCache.find(build => {
					const expectedDirName = this.sanitizeBuildName(build);
					return dir === expectedDirName;
				});

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

		return extractedBuilds;
	}

	/**
	 * Clean up old builds
	 */
	async cleanupOldBuilds(): Promise<{ removedDownloads: number; removedExtracts: number }> {
		const maxBuilds = this.settings.maxBuildsToKeep;
		
		const removedDownloads = await this.cleanupOldDownloads(maxBuilds);
		const removedExtracts = await this.downloader.cleanupOldBuilds(this.getExtractsPath(), maxBuilds);

		if (this.settings.showNotifications && (removedDownloads > 0 || removedExtracts > 0)) {
			new Notice(`Cleaned up ${removedDownloads} downloads and ${removedExtracts} extracts`);
		}

		return { removedDownloads, removedExtracts };
	}

	/**
	 * Clean up old downloads
	 */
	private async cleanupOldDownloads(maxBuilds: number): Promise<number> {
		const downloadsPath = this.getDownloadsPath();
		if (!fs.existsSync(downloadsPath)) {
			return 0;
		}

		const files = fs.readdirSync(downloadsPath)
			.map(name => ({
				name,
				path: path.join(downloadsPath, name),
				stats: fs.statSync(path.join(downloadsPath, name))
			}))
			.filter(item => item.stats.isFile())
			.sort((a, b) => b.stats.mtime.getTime() - a.stats.mtime.getTime());

		if (files.length <= maxBuilds) {
			return 0;
		}

		const filesToRemove = files.slice(maxBuilds);
		let removedCount = 0;

		for (const file of filesToRemove) {
			try {
				fs.unlinkSync(file.path);
				removedCount++;
			} catch (error) {
				console.error(`Failed to remove file ${file.name}:`, error);
			}
		}

		return removedCount;
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
				console.log(`Cleaned up empty directory: ${dirPath}`);
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
		
		if (newBuilds.length > 0 && this.settings.showNotifications) {
			new Notice(`Found ${newBuilds.length} new Blender builds`);
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
			console.log('No cached builds found or cache invalid, will need to refresh');
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
				console.log('Cache version mismatch, invalidating cache');
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
				console.log(`Loaded ${builds.length} builds from cache`);
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
			console.log(`Cached ${builds.length} builds to disk`);
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
				console.log('Build cache cleared');
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
}
