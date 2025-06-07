import { BlenderBuildInfo, DownloadProgress, ExtractionProgress } from './types';
import { requestUrl } from 'obsidian';
import * as fs from 'fs';
import * as path from 'path';
import { promisify } from 'util';
import * as yauzl from 'yauzl';
import { EventEmitter } from 'events';
import { BlenderExtractor } from './extractor';
import { 
	debug, 
	info, 
	warn, 
	error,
	registerLoggerClass 
} from './utils/obsidian-logger';

export class BlenderDownloader extends EventEmitter {
	private downloadQueue: Map<string, AbortController> = new Map();
	private progressCallbacks: Map<string, (progress: DownloadProgress) => void> = new Map();
	private extractor: BlenderExtractor;
	constructor() {
		super();
		registerLoggerClass(this, 'BlenderDownloader');
		debug(this, 'BlenderDownloader constructor started');
		
		this.extractor = new BlenderExtractor();
		
		// Forward extraction events
		debug(this, 'Setting up extractor event forwarding');
		this.extractor.on('extractionStarted', (archivePath: string, extractPath: string) => {
			debug(this, `Extraction started: ${archivePath} -> ${extractPath}`);
			this.emit('extractionStarted', archivePath, extractPath);
		});
		
		this.extractor.on('extractionCompleted', (archivePath: string, extractPath: string) => {
			info(this, `Extraction completed: ${archivePath} -> ${extractPath}`);
			this.emit('extractionCompleted', archivePath, extractPath);
		});
		
		this.extractor.on('extractionError', (archivePath: string, errorData: Error) => {
			error(this, `Extraction failed for ${archivePath}`, errorData);
			this.emit('extractionError', archivePath, errorData);
		});
		
		this.extractor.on('extractionProgress', (progress: ExtractionProgress) => {
			debug(this, `Extraction progress: ${progress.percentage}% (${progress.extractedFiles}/${progress.totalFiles} files)`);
			this.emit('extractionProgress', progress);
		});
		
		info(this, 'BlenderDownloader constructor completed');
	}

	/**
	 * Download a Blender build
	 */
    async downloadBuild(
		build: BlenderBuildInfo, 
		downloadPath: string, 
		onProgress?: (progress: DownloadProgress) => void
	): Promise<string> {
		const buildId = this.generateBuildId(build);
		const fileName = this.extractFileName(build.link);
		const fullPath = path.join(downloadPath, fileName);
				// Create download directory if it doesn't exist
		if (!fs.existsSync(path.dirname(fullPath))) {
			fs.mkdirSync(path.dirname(fullPath), { recursive: true });
		}

		// Set up progress callback
		if (onProgress) {
			this.progressCallbacks.set(buildId, onProgress);
		}
		// Create abort controller for this download
		const abortController = new AbortController();
		this.downloadQueue.set(buildId, abortController);
		
		// Emit download started event
		this.emit('downloadStarted', build, fullPath);
		
		try {
			// Get file size first
			let totalSize = 0;
			try {
				const headResponse = await requestUrl({
					url: build.link,
					method: 'HEAD'
				});
				totalSize = parseInt(headResponse.headers['content-length'] || '0');
			} catch (error) {
				// If HEAD request fails, continue without size info
				console.warn('Could not get file size:', error);
			}
            const progress: DownloadProgress = {
				downloaded: 0,
				total: totalSize,
				percentage: 0,
				speed: 0,
				status: 'downloading'
			};

			this.notifyProgress(buildId, progress);

			// Use Obsidian's requestUrl for downloading
			const response = await requestUrl({
				url: build.link,
				method: 'GET'
			});

			// Write the file
			fs.writeFileSync(fullPath, Buffer.from(response.arrayBuffer));

			progress.downloaded = response.arrayBuffer.byteLength;
			progress.total = progress.total || progress.downloaded;			progress.percentage = 100;
			progress.status = 'completed';
			this.notifyProgress(buildId, progress);

			// Emit download completed event
			this.emit('downloadCompleted', build, fullPath);

			return fullPath;
		} catch (error) {
			const progress: DownloadProgress = {
				downloaded: 0,
				total: 0,
				percentage: 0,
				speed: 0,
				status: 'error',
				error: error instanceof Error ? error.message : 'Unknown error'			};
			
			this.notifyProgress(buildId, progress);
			
			// Emit download error event
			this.emit('downloadError', build, error);
			
			throw error;
		} finally {
			this.downloadQueue.delete(buildId);
			this.progressCallbacks.delete(buildId);
		}
	}

	/**
	 * Cancel a download
	 */
	cancelDownload(buildId: string): boolean {
		const abortController = this.downloadQueue.get(buildId);
		if (abortController) {
			abortController.abort();
			this.downloadQueue.delete(buildId);
			this.progressCallbacks.delete(buildId);
			return true;
		}
		return false;
	}

	/**
	 * Get active downloads
	 */
	getActiveDownloads(): string[] {
		return Array.from(this.downloadQueue.keys());
	}

	/**
	 * Extract file name from URL
	 */
	private extractFileName(url: string): string {
		const urlParts = url.split('/');
		return urlParts[urlParts.length - 1] || 'blender-build.zip';
	}

	/**
	 * Generate unique build ID
	 */	private generateBuildId(build: BlenderBuildInfo): string {
		const hash = build.buildHash || 'unknown';
		return `${build.subversion}-${build.branch}-${hash}`;
	}

	/**
	 * Notify progress callback
	 */
	private notifyProgress(buildId: string, progress: DownloadProgress): void {
		const callback = this.progressCallbacks.get(buildId);
		if (callback) {
			callback(progress);
		}
	}	/**
	 * Extract downloaded archive using the BlenderExtractor
	 */
	async extractBuild(archivePath: string, extractPath: string): Promise<string> {
		return this.extractor.extractBuild(archivePath, extractPath);
	}

	/**
	 * Clean up old builds
	 */
	async cleanupOldBuilds(buildsPath: string, maxBuilds: number): Promise<number> {
		const fs = require('fs');
		const path = require('path');
		
		if (!fs.existsSync(buildsPath)) {
			return 0;
		}

		const builds = fs.readdirSync(buildsPath)
			.map((name: string) => ({
				name,
				path: path.join(buildsPath, name),
				stats: fs.statSync(path.join(buildsPath, name))
			}))
			.filter((item: any) => item.stats.isDirectory())
			.sort((a: any, b: any) => b.stats.mtime.getTime() - a.stats.mtime.getTime());

		if (builds.length <= maxBuilds) {
			return 0;
		}

		const buildsToRemove = builds.slice(maxBuilds);
		let removedCount = 0;

		for (const build of buildsToRemove) {
			try {
				await this.removeDirectory(build.path);
				removedCount++;
			} catch (error) {
				console.error(`Failed to remove build ${build.name}:`, error);
			}
		}

		return removedCount;
	}

	/**
	 * Remove directory recursively
	 */
	private async removeDirectory(dirPath: string): Promise<void> {
		const fs = require('fs');
		const path = require('path');
		const { promisify } = require('util');
		const rmdir = promisify(fs.rmdir);
		const unlink = promisify(fs.unlink);
		const readdir = promisify(fs.readdir);
		const stat = promisify(fs.stat);

		const items = await readdir(dirPath);
		
		for (const item of items) {
			const itemPath = path.join(dirPath, item);
			const itemStat = await stat(itemPath);
			
			if (itemStat.isDirectory()) {
				await this.removeDirectory(itemPath);
			} else {
				await unlink(itemPath);
			}
		}
		
		await rmdir(dirPath);
	}
	/**
	 * Get extracted build directory using the BlenderExtractor
	 */
	findBlenderExecutable(extractedPath: string): string | null {
		return this.extractor.findBlenderExecutable(extractedPath);
	}
}
