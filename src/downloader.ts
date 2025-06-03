import { BlenderBuildInfo, DownloadProgress, ExtractionProgress } from './types';
import { requestUrl } from 'obsidian';
import * as fs from 'fs';
import * as path from 'path';
import { exec } from 'child_process';
import { promisify } from 'util';
import * as yauzl from 'yauzl';
import { EventEmitter } from 'events';

export class BlenderDownloader extends EventEmitter {
	private downloadQueue: Map<string, AbortController> = new Map();
	private progressCallbacks: Map<string, (progress: DownloadProgress) => void> = new Map();

	constructor() {
		super();
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
			progress.total = progress.total || progress.downloaded;
			progress.percentage = 100;
			progress.status = 'completed';
			this.notifyProgress(buildId, progress);

			return fullPath;
		} catch (error) {
			const progress: DownloadProgress = {
				downloaded: 0,
				total: 0,
				percentage: 0,
				speed: 0,
				status: 'error',
				error: error instanceof Error ? error.message : 'Unknown error'
			};
			
			this.notifyProgress(buildId, progress);
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
	}

	/**
	 * Extract downloaded archive
	 */
	async extractBuild(archivePath: string, extractPath: string): Promise<string> {
		const path = require('path');
		const fs = require('fs');
		
		// Create extraction directory
		if (!fs.existsSync(extractPath)) {
			fs.mkdirSync(extractPath, { recursive: true });
		}

		const fileName = path.basename(archivePath);
		const isZip = fileName.endsWith('.zip');
		const isTarGz = fileName.endsWith('.tar.gz');
		const isDmg = fileName.endsWith('.dmg');

		if (isZip) {
			return this.extractZip(archivePath, extractPath);
		} else if (isTarGz) {
			return this.extractTarGz(archivePath, extractPath);
		} else if (isDmg) {
			return this.extractDmg(archivePath, extractPath);
		} else {
			throw new Error(`Unsupported archive format: ${fileName}`);
		}
	}

	/**
	 * Extract ZIP archive (Windows)
	 */
	private async extractZip(archivePath: string, extractPath: string): Promise<string> {
		const { exec } = require('child_process');
		const { promisify } = require('util');
		const execAsync = promisify(exec);

		try {
			// Use PowerShell's Expand-Archive on Windows
			const command = `powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${extractPath}' -Force"`;
			await execAsync(command);
			return extractPath;
		} catch (error) {
			throw new Error(`Failed to extract ZIP: ${error}`);
		}
	}

	/**
	 * Extract TAR.GZ archive (Linux/macOS)
	 */
	private async extractTarGz(archivePath: string, extractPath: string): Promise<string> {
		const { exec } = require('child_process');
		const { promisify } = require('util');
		const execAsync = promisify(exec);

		try {
			const command = `tar -xzf "${archivePath}" -C "${extractPath}"`;
			await execAsync(command);
			return extractPath;
		} catch (error) {
			throw new Error(`Failed to extract TAR.GZ: ${error}`);
		}
	}

	/**
	 * Extract DMG archive (macOS)
	 */
	private async extractDmg(archivePath: string, extractPath: string): Promise<string> {
		const { exec } = require('child_process');
		const { promisify } = require('util');
		const execAsync = promisify(exec);

		try {
			// Mount the DMG
			const mountResult = await execAsync(`hdiutil attach "${archivePath}"`);
			const mountPoint = mountResult.stdout.trim().split('\t').pop();
			
			// Copy contents
			await execAsync(`cp -R "${mountPoint}"/* "${extractPath}"/`);
			
			// Unmount
			await execAsync(`hdiutil detach "${mountPoint}"`);
			
			return extractPath;
		} catch (error) {
			throw new Error(`Failed to extract DMG: ${error}`);
		}
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
	 * Get extracted build directory
	 */
	findBlenderExecutable(extractedPath: string): string | null {
		const findExecutable = (dir: string): string | null => {
			try {
				const items = fs.readdirSync(dir);
				
				for (const item of items) {
					const itemPath = path.join(dir, item);
					const stats = fs.statSync(itemPath);
					
					if (stats.isDirectory()) {
						const result = findExecutable(itemPath);
						if (result) return result;
					} else if (stats.isFile()) {
						// Look for blender executable
						const isExecutable = process.platform === 'win32' 
							? item.toLowerCase() === 'blender.exe'
							: item === 'blender' || item === 'Blender';
						
						if (isExecutable) {
							return itemPath;
						}
					}
				}
			} catch (error) {
				console.error('Error searching for executable:', error);
			}
			
			return null;
		};

		return findExecutable(extractedPath);
	}
}
