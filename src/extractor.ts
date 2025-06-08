import { ExtractionProgress } from './types';
import { EventEmitter } from 'events';
import { Platform } from 'obsidian';
import * as fs from 'fs';
import * as path from 'path';
import { exec } from 'child_process';

/**
 * Handles extraction of Blender build archives (ZIP, TAR.GZ, DMG)
 * with progress monitoring and verification
 */
export class BlenderExtractor extends EventEmitter {
	private currentProgressMonitor: NodeJS.Timeout | null = null;

	constructor() {
		super();
	}

	/**
	 * Extract downloaded archive
	 */
	async extractBuild(archivePath: string, extractPath: string): Promise<string> {
		// Emit extraction started event
		this.emit('extractionStarted', archivePath, extractPath);
		
		try {
			// Create extraction directory
			if (!fs.existsSync(extractPath)) {
				fs.mkdirSync(extractPath, { recursive: true });
			}

			const fileName = path.basename(archivePath);
			const isZip = fileName.endsWith('.zip');
			const isTarGz = fileName.endsWith('.tar.gz');
			const isDmg = fileName.endsWith('.dmg');

			let result: string;
			if (isZip) {
				result = await this.extractZip(archivePath, extractPath);
			} else if (isTarGz) {
				result = await this.extractTarGz(archivePath, extractPath);
			} else if (isDmg) {
				result = await this.extractDmg(archivePath, extractPath);
			} else {
				throw new Error(`Unsupported archive format: ${fileName}`);
			}
			
			// Note: extractionCompleted event is now emitted from individual extraction methods
			// after verification is complete
			
			return result;
		} catch (error) {
			// Emit extraction error event
			this.emit('extractionError', archivePath, error);
			throw error;
		}
	}

	/**
	 * Extract ZIP archive (Windows)
	 */
	private async extractZip(archivePath: string, extractPath: string): Promise<string> {
		return new Promise((resolve, reject) => {
			// Use PowerShell's Expand-Archive on Windows
			const command = `powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${extractPath}' -Force"`;
			
			const process = exec(command, (error: any, stdout: string, stderr: string) => {
				if (error) {
					reject(new Error(`Failed to extract ZIP: ${error.message}`));
					return;
				}
				
				if (stderr) {
					console.warn(`Extraction warning: ${stderr}`);
				}
				
				// Verify extraction completed by checking if files exist
				this.verifyExtractionComplete(extractPath)
					.then(() => {
						// Emit completion event only after verification
						this.emit('extractionCompleted', archivePath, extractPath);
						resolve(extractPath);
					})
					.catch(reject);
			});
			
			// Start monitoring extraction progress after a short delay
			setTimeout(() => {
				this.monitorExtractionProgress(extractPath, archivePath);
			}, 1000);
			
			// Set a reasonable timeout (10 minutes for large archives)
			const timeout = setTimeout(() => {
				process.kill();
				reject(new Error('Extraction timed out after 10 minutes'));
			}, 10 * 60 * 1000);
			
			process.on('exit', () => {
				clearTimeout(timeout);
			});
		});
	}

	/**
	 * Extract TAR.GZ archive (Linux/macOS)
	 */
	private async extractTarGz(archivePath: string, extractPath: string): Promise<string> {
		return new Promise((resolve, reject) => {
			const command = `tar -xzf "${archivePath}" -C "${extractPath}"`;
			
			const process = exec(command, (error: any, stdout: string, stderr: string) => {
				if (error) {
					reject(new Error(`Failed to extract TAR.GZ: ${error.message}`));
					return;
				}
				
				if (stderr) {
					console.warn(`Extraction warning: ${stderr}`);
				}
				
				// Verify extraction completed by checking if files exist
				this.verifyExtractionComplete(extractPath)
					.then(() => {
						// Emit completion event only after verification
						this.emit('extractionCompleted', archivePath, extractPath);
						resolve(extractPath);
					})
					.catch(reject);
			});
			
			// Start monitoring extraction progress after a short delay
			setTimeout(() => {
				this.monitorExtractionProgress(extractPath, archivePath);
			}, 1000);
			
			// Set a reasonable timeout (10 minutes for large archives)
			const timeout = setTimeout(() => {
				process.kill();
				reject(new Error('Extraction timed out after 10 minutes'));
			}, 10 * 60 * 1000);
			
			process.on('exit', () => {
				clearTimeout(timeout);
			});
		});
	}

	/**
	 * Extract DMG archive (macOS)
	 */
	private async extractDmg(archivePath: string, extractPath: string): Promise<string> {
		return new Promise((resolve, reject) => {
			// Mount the DMG first
			const mountCommand = `hdiutil attach "${archivePath}"`;
			
			exec(mountCommand, (mountError: any, mountStdout: string) => {
				if (mountError) {
					reject(new Error(`Failed to mount DMG: ${mountError.message}`));
					return;
				}
				
				const mountPoint = mountStdout.trim().split('\t').pop();
				
				// Start monitoring extraction progress after mounting
				setTimeout(() => {
					this.monitorExtractionProgress(extractPath, archivePath);
				}, 1000);
				
				// Copy contents
				const copyCommand = `cp -R "${mountPoint}"/* "${extractPath}"/`;
				exec(copyCommand, (copyError: any, copyStdout: string, copyStderr: string) => {
					// Always try to unmount, regardless of copy success
					const unmountCommand = `hdiutil detach "${mountPoint}"`;
					exec(unmountCommand, (unmountError: any) => {
						if (unmountError) {
							console.warn(`Warning: Failed to unmount DMG: ${unmountError.message}`);
						}
						
						if (copyError) {
							reject(new Error(`Failed to copy DMG contents: ${copyError.message}`));
							return;
						}
						
						if (copyStderr) {
							console.warn(`Copy warning: ${copyStderr}`);
						}
						
						// Verify extraction completed
						this.verifyExtractionComplete(extractPath)
							.then(() => {
								// Emit completion event only after verification
								this.emit('extractionCompleted', archivePath, extractPath);
								resolve(extractPath);
							})
							.catch(reject);
					});
				});
			});
		});
	}

	/**
	 * Find Blender executable in extracted directory
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
						const isExecutable = Platform.isWin 
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

	/**
	 * Verify that extraction has completed by checking for expected files
	 */
	private async verifyExtractionComplete(extractPath: string): Promise<void> {
		// Stop any active progress monitoring since we're now in verification phase
		if (this.currentProgressMonitor) {
			clearInterval(this.currentProgressMonitor);
			this.currentProgressMonitor = null;
		}
		
		const maxAttempts = 10; // Try up to 10 times
		const checkInterval = 2000; // Check every 2 seconds
		
		for (let attempt = 1; attempt <= maxAttempts; attempt++) {
			// Wait before checking
			await new Promise(resolve => setTimeout(resolve, checkInterval));
			
			// Check if the extraction directory exists and has content
			if (!fs.existsSync(extractPath)) {
				if (attempt === maxAttempts) {
					throw new Error('Extraction directory does not exist');
				}
				continue;
			}
			
			const contents = fs.readdirSync(extractPath);
			if (contents.length === 0) {
				if (attempt === maxAttempts) {
					throw new Error('Extraction directory is empty');
				}
				continue;
			}
			
			// Look for typical Blender directory structure or executable
			let foundBlenderStructure = false;
			let directoryStats: { totalSize: number, fileCount: number } | null = null;
			
			try {
				directoryStats = this.getDirectoryStats(extractPath);
			} catch (error) {
				// If we can't get stats, filesystem might still be settling
				if (attempt === maxAttempts) {
					console.warn('Warning: Could not get directory stats during verification');
				}
				continue;
			}
			
			// Check if we have a reasonable amount of content (at least 50MB for Blender)
			if (directoryStats && directoryStats.totalSize < 50 * 1024 * 1024) {
				console.log(`Extraction verification attempt ${attempt}: Only ${this.formatBytes(directoryStats.totalSize)} extracted, waiting for more...`);
				if (attempt === maxAttempts) {
					console.warn('Warning: Extracted content seems smaller than expected for Blender');
				} else {
					continue;
				}
			}
			
			// Look for Blender executable or typical directory structure
			for (const item of contents) {
				const itemPath = path.join(extractPath, item);
				const stats = fs.statSync(itemPath);
				
				if (stats.isDirectory()) {
					// Check if this looks like a Blender directory
					const blenderExecutable = this.findBlenderExecutable(itemPath);
					if (blenderExecutable) {
						foundBlenderStructure = true;
						break;
					}
				}
			}
			
			if (foundBlenderStructure) {
				const sizeInfo = directoryStats ? this.formatBytes(directoryStats.totalSize) : 'unknown size';
				console.log(`Extraction verification successful on attempt ${attempt}: Found Blender structure with ${sizeInfo} total`);
				return; // Verification successful
			}
			
			if (attempt === maxAttempts) {
				console.warn('Warning: Could not verify Blender executable in extracted files after all attempts');
				// Don't throw error here as the extraction might still be valid for some builds
				return;
			} else {
				console.log(`Extraction verification attempt ${attempt}: No Blender structure found yet, retrying...`);
			}
		}
	}

	/**
	 * Monitor extraction progress by checking directory size
	 */
	private monitorExtractionProgress(extractPath: string, archivePath: string): void {
		let lastSize = 0;
		let stableCount = 0;
		const checkInterval = 3000; // Check every 3 seconds
		const maxStableChecks = 5; // Consider stable after 5 consecutive unchanged size checks (15 seconds)
		let monitoringActive = true;
		
		const progressCheck = setInterval(() => {
			try {
				if (!monitoringActive) {
					clearInterval(progressCheck);
					return;
				}
				
				if (!fs.existsSync(extractPath)) {
					return; // Directory doesn't exist yet
				}
				
				// Get directory size (rough estimation)
				const stats = this.getDirectoryStats(extractPath);
				const currentSize = stats.totalSize;
				
				if (currentSize === lastSize && lastSize > 0) {
					stableCount++;
					
					// Emit progress updates showing we're still checking
					this.emit('extractionProgress', {
						archivePath,
						extractPath,
						progress: -1, // Indeterminate progress
						status: `Verifying extraction... (${this.formatBytes(currentSize)} extracted)`
					});
					
					if (stableCount >= maxStableChecks) {
						// Size has been stable for a while, extraction likely complete
						monitoringActive = false;
						clearInterval(progressCheck);
						this.emit('extractionProgress', {
							archivePath,
							extractPath,
							progress: 100,
							status: 'Finalizing extraction...'
						});
					}
				} else {
					stableCount = 0;
					lastSize = currentSize;
					
					// Emit progress update (we can't calculate exact percentage without knowing final size)
					this.emit('extractionProgress', {
						archivePath,
						extractPath,
						progress: -1, // Indeterminate progress
						status: `Extracting... (${this.formatBytes(currentSize)} extracted)`
					});
				}
			} catch (error) {
				// If there's an error checking, just continue
				console.warn('Error monitoring extraction progress:', error);
			}
		}, checkInterval);
		
		// Store reference to stop monitoring when extraction completes
		this.currentProgressMonitor = progressCheck;
		
		// Stop monitoring after 20 minutes max
		setTimeout(() => {
			monitoringActive = false;
			clearInterval(progressCheck);
		}, 20 * 60 * 1000);
	}
	
	/**
	 * Get directory statistics (size, file count)
	 */
	private getDirectoryStats(dirPath: string): { totalSize: number, fileCount: number } {
		let totalSize = 0;
		let fileCount = 0;
		
		try {
			const items = fs.readdirSync(dirPath);
			
			for (const item of items) {
				const itemPath = path.join(dirPath, item);
				const stats = fs.statSync(itemPath);
				
				if (stats.isDirectory()) {
					const subStats = this.getDirectoryStats(itemPath);
					totalSize += subStats.totalSize;
					fileCount += subStats.fileCount;
				} else {
					totalSize += stats.size;
					fileCount++;
				}
			}
		} catch (error) {
			// If there's an error, return current totals
		}
		
		return { totalSize, fileCount };
	}
	
	/**
	 * Format bytes to human readable string
	 */
	private formatBytes(bytes: number): string {
		if (bytes === 0) return '0 B';
		
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
	}
}
