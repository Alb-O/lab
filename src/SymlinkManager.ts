import * as fs from 'fs';
import * as path from 'path';
import { Platform, Notice } from 'obsidian';
import { BlenderBuildInfo } from './types';
import { SYMLINK_NAME } from './constants';

export class SymlinkManager {
	private extractsPath: string;

	constructor(extractsPath: string) {
		this.extractsPath = extractsPath;
	}

	/**
	 * Get the full path to the symlink
	 */
	getSymlinkPath(): string {
		return path.join(this.extractsPath, SYMLINK_NAME);
	}

	/**
	 * Check if the symlink exists and is valid
	 */
	isSymlinkValid(): boolean {
		const symlinkPath = this.getSymlinkPath();
		
		try {
			const stats = fs.lstatSync(symlinkPath);
			if (stats.isSymbolicLink() || (Platform.isWin && stats.isDirectory())) {
				// Check if the target still exists
				return fs.existsSync(symlinkPath);
			}
			return false;
		} catch (error: any) {
			if (error.code === 'ENOENT') {
				return false;
			}
			throw error;
		}
	}

	/**
	 * Get the target path of the current symlink
	 */
	getSymlinkTarget(): string | null {
		const symlinkPath = this.getSymlinkPath();
		
		try {
			if (this.isSymlinkValid()) {
				return fs.readlinkSync(symlinkPath);
			}
			return null;
		} catch (error) {
			return null;
		}
	}

	/**
	 * Remove existing symlink if it exists
	 */
	private removeExistingSymlink(): void {
		const symlinkPath = this.getSymlinkPath();

		try {
			const stats = fs.lstatSync(symlinkPath);
			// If lstatSync succeeds, something exists at this path
			if (stats.isSymbolicLink() || (Platform.isWin && stats.isDirectory())) {
				// On Windows, junctions appear as directories but can be safely unlinked
				// On other platforms, check for symbolic links
				if (Platform.isWin) {
					// Use rmSync for Windows junctions as they can be stubborn
					fs.rmSync(symlinkPath, { recursive: false, force: true });
				} else {
					fs.unlinkSync(symlinkPath);
				}
			} else {
				throw new Error(`${SYMLINK_NAME} exists but is not a symlink - cannot replace`);
			}
		} catch (error: any) {
			// If lstatSync throws ENOENT, the path doesn't exist, which is fine
			if (error.code !== 'ENOENT') {
				throw new Error(`Failed to remove existing ${SYMLINK_NAME}: ${error.message}`);
			}
		}
	}

	/**
	 * Create a symlink pointing to the specified build path
	 */
	async createSymlink(buildPath: string, build: BlenderBuildInfo): Promise<void> {
		if (!fs.existsSync(buildPath)) {
			throw new Error('Build path does not exist on filesystem');
		}

		// Remove existing symlink if it exists
		this.removeExistingSymlink();

		const symlinkPath = this.getSymlinkPath();

		try {
			// Create the symlink - use platform-appropriate type
			const symlinkType = Platform.isWin ? 'junction' : 'dir';
			fs.symlinkSync(buildPath, symlinkPath, symlinkType);
			
			new Notice(`Created symlink: ${SYMLINK_NAME} -> ${build.subversion}`);
		} catch (error: any) {
			throw new Error(`Failed to create symlink: ${error.message}`);
		}
	}

	/**
	 * Remove the symlink
	 */
	async removeSymlink(): Promise<void> {
		try {
			this.removeExistingSymlink();
			new Notice(`Removed symlink: ${SYMLINK_NAME}`);
		} catch (error: any) {
			throw new Error(`Failed to remove symlink: ${error.message}`);
		}
	}

	/**
	 * Update symlink to point to a different build
	 */
	async updateSymlink(buildPath: string, build: BlenderBuildInfo): Promise<void> {
		await this.createSymlink(buildPath, build);
	}
}
