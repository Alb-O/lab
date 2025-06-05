import { ButtonComponent, setTooltip, Notice } from 'obsidian';
import { BlenderBuildInfo } from '../../types';
import { FetchBlenderBuilds } from '../../buildManager';
import type FetchBlenderBuildsPlugin from '../../main';

export class BlenderBuildsRenderer {
	private plugin: FetchBlenderBuildsPlugin;
	private buildManager: FetchBlenderBuilds;
	private onRefresh: () => void;

	constructor(
		plugin: FetchBlenderBuildsPlugin,
		buildManager: FetchBlenderBuilds,
		onRefresh: () => void
	) {
		this.plugin = plugin;
		this.buildManager = buildManager;
		this.onRefresh = onRefresh;
	}
	/**
	 * Render builds list in container
	 */
	renderBuilds(
		container: HTMLElement, 
		builds: BlenderBuildInfo[], 
		searchFilter?: string
	): void {
		container.empty();
		
		if (builds.length === 0) {
			this.renderEmptyState(container);
			return;
		}

		const buildsList = container.createEl('div', { cls: 'blender-builds-list' });
		
		builds.forEach((build, index) => {
			this.createBuildItem(buildsList, build, index, searchFilter);
		});
	}
	
	/**
	 * Create a single build item - compact two-line design
	 */
	private createBuildItem(
		buildsList: HTMLElement, 
		build: BlenderBuildInfo, 
		index: number,
		searchFilter?: string,
		// Removed highlightFunction, will use internal helper
	): void {
		const listItem = buildsList.createEl('div', { cls: 'blender-build-item' });
		
		// Main content area
		const contentEl = listItem.createEl('div', { cls: 'blender-build-content' });
		// First line: Version, Branch, Date, Hash
		const mainLineEl = contentEl.createEl('div', { cls: 'blender-build-main-line' });
		
		// Helper to apply highlighting
		const highlight = (text: string) => searchFilter ? this.highlightText(searchFilter, text) : text;

		// Version (more prominent) - with highlighting
		const versionEl = mainLineEl.createEl('span', { 
			cls: 'blender-build-version'
		});
		versionEl.innerHTML = highlight(build.subversion);
		setTooltip(versionEl, `Version: ${build.subversion}`);

		// Branch tag - with highlighting
		const branchEl = mainLineEl.createEl('span', { 
			cls: `blender-branch-tag branch-${build.branch.toLowerCase()}`
		});
		branchEl.innerHTML = highlight(build.branch.toUpperCase());
		setTooltip(branchEl, `Branch: ${build.branch}`);
		
		// Build hash - with highlighting
		if (build.buildHash) {
			const hashEl = mainLineEl.createEl('span', { 
				cls: 'blender-build-hash'
			});
			hashEl.innerHTML = highlight(build.buildHash.substring(0, 8));
			setTooltip(hashEl, `Build hash: ${build.buildHash}`);
		}

		// Date - with highlighting
		const dateEl = mainLineEl.createEl('span', { 
			cls: 'blender-build-date'
		});
		dateEl.innerHTML = highlight(build.commitTime.toLocaleDateString());
		setTooltip(dateEl, `Committed: ${build.commitTime.toLocaleString()}`);

		// Second line: Filename only
		const detailsLineEl = contentEl.createEl('div', { cls: 'blender-build-details-line' });
		
		// Filename - with highlighting
		const filename = this.getFilenameFromUrl(build.link);
		const filenameEl = detailsLineEl.createEl('span', { 
			cls: 'blender-build-filename'
		});
		filenameEl.innerHTML = highlight(filename);
		setTooltip(filenameEl, `File: ${filename}`);
		// Check if build is installed to determine clickable behavior
		const installStatus = this.buildManager.isBuildInstalled(build);
		const isInstalled = installStatus.downloaded || installStatus.extracted;

		// Add action buttons (pass install status to avoid duplicate calls)
		const actionsEl = listItem.createEl('div', { cls: 'blender-build-actions' });
		this.addBuildActions(actionsEl, build, index, installStatus);

		// Make the entire item clickable for download only if not installed
		if (isInstalled) {
			// For installed builds, add a different visual style
			listItem.addClass('blender-build-installed');
		}
	}
	
	/**
	 * Add action buttons for a build item
	 */
	private addBuildActions(actionsEl: HTMLElement, build: BlenderBuildInfo, index: number, installStatus: { downloaded: boolean; extracted: boolean }): void {
		const isInstalled = installStatus.downloaded || installStatus.extracted;
		
		if (isInstalled) {
			// For installed builds: Launch button (first), Extract button (if needed), Trash button (last)
			
			// Launch button - show for all installed builds, but only enable if extracted
			const launchBtn = new ButtonComponent(actionsEl)
				.setIcon('play')
				.setTooltip(installStatus.extracted ? 'Launch Blender' : 'Extract build first to launch')
				.setClass('clickable-icon')
				.setClass('blender-action-button')
				.setClass('blender-launch-button');
			
			if (installStatus.extracted) {
				launchBtn.buttonEl.addEventListener('click', async (evt) => {
					evt.preventDefault();
					evt.stopPropagation();
					try {
						await this.launchBuild(build);
					} catch (error) {
						console.error('Failed to launch build:', error);
						new Notice(`Failed to launch ${build.subversion}: ${error.message}`);
					}
				});
			} else {
				// Disable the button if not extracted
				launchBtn.setDisabled(true);
			}
			
			// Extract button - show only if downloaded but not extracted and auto-extract is off
			if (installStatus.downloaded && !installStatus.extracted && !this.plugin.settings.autoExtract) {
				const extractBtn = new ButtonComponent(actionsEl)
					.setIcon('folder-open')
					.setTooltip('Extract this build')
					.setClass('clickable-icon')
					.setClass('blender-action-button')
					.setClass('blender-extract-button');
				extractBtn.buttonEl.addEventListener('click', async (evt) => {
					evt.preventDefault();
					evt.stopPropagation();
					try {
						await this.extractBuild(build);
					} catch (error) {
						console.error('Failed to extract build:', error);
						new Notice(`Failed to extract ${build.subversion}: ${error.message}`);
					}
				});
			}

			// Symlink button - show only for extracted builds
			if (installStatus.extracted) {
				const symlinkBtn = new ButtonComponent(actionsEl)
					.setIcon('link')
					.setTooltip('Create symlink to this build')
					.setClass('clickable-icon')
					.setClass('blender-action-button')
					.setClass('blender-symlink-button');
				symlinkBtn.buttonEl.addEventListener('click', async (evt) => {
					evt.preventDefault();
					evt.stopPropagation();
					try {
						await this.symlinkBuild(build);
					} catch (error) {
						console.error('Failed to symlink build:', error);
						new Notice(`Failed to symlink ${build.subversion}: ${error.message}`);
					}
				});
			}
			
			// Trash button - for all installed builds
			const trashBtn = new ButtonComponent(actionsEl)
				.setIcon('trash-2')
				.setTooltip('Delete this build')
				.setClass('clickable-icon')
				.setClass('blender-action-button')
				.setClass('blender-trash-button');
			trashBtn.buttonEl.addEventListener('click', async (evt) => {
				evt.preventDefault();
				evt.stopPropagation();
				try {
					await this.deleteBuild(build);
				} catch (error) {
					console.error('Failed to delete build:', error);
					new Notice(`Failed to delete ${build.subversion}: ${error.message}`);
				}
			});
		} else {
			// For non-installed builds: Download button (first position)
			const downloadBtn = new ButtonComponent(actionsEl)
				.setIcon('download')
				.setTooltip('Download this build')
				.setClass('clickable-icon')
				.setClass('blender-action-button');
			downloadBtn.buttonEl.addEventListener('click', async (evt) => {
				evt.preventDefault();
				evt.stopPropagation();
				try {
					await this.downloadBuild(build);
				} catch (error) {
					console.error('Failed to download build:', error);
					new Notice(`Failed to download ${build.subversion}: ${error.message}`);
				}
			});
		}
	}

	/**
	 * Render empty state when no builds are available
	 */
	private renderEmptyState(container: HTMLElement): void {
		const emptyState = container.createEl('div', { cls: 'blender-empty-state' });
		
		const emptyIcon = emptyState.createEl('div', { 
			cls: 'blender-empty-icon',
			text: '📦' 
		});
		
		const emptyMessage = emptyState.createEl('div', { 
			cls: 'blender-empty-message',
			text: 'No Blender builds found'
		});
		
		const emptySubtext = emptyState.createEl('div', { 
			cls: 'blender-empty-subtext',
			text: 'Try refreshing or adjusting your filters'
		});
	}

	/**
	 * Download a build
	 */
	private async downloadBuild(build: BlenderBuildInfo): Promise<void> {
		try {
			await this.buildManager.downloadBuild(build);
			// The buildManager handles all notifications via events
		} catch (error) {
			console.error('Failed to download build:', error);
			new Notice(`Failed to start download: ${error.message}`);
		}
	}
	/**
	 * Delete a build
	 */
	private async deleteBuild(build: BlenderBuildInfo): Promise<void> {
		try {
			await this.buildManager.deleteBuild(build);
			// Refresh the view to update the UI
			this.onRefresh();
		} catch (error) {
			console.error('Failed to delete build:', error);
			new Notice(`Failed to delete build: ${error.message}`);
		}
	}	/**
	 * Extract a build
	 */
	private async extractBuild(build: BlenderBuildInfo): Promise<void> {
		try {
			// Get the archive path for the build (same logic as isBuildInstalled)
			const downloadsPath = this.buildManager.getDownloadsPathForBuild(build);
			const expectedFileName = this.extractFileName(build.link);
			const path = require('path');
			const archivePath = path.join(downloadsPath, expectedFileName);
			
			await this.buildManager.extractBuild(archivePath, build);
			// Refresh the view to update the UI
			this.onRefresh();
		} catch (error) {
			console.error('Failed to extract build:', error);
			new Notice(`Failed to extract build: ${error.message}`);
		}
	}

	/**
	 * Extract filename from URL (helper method)
	 */
	private extractFileName(url: string): string {
		try {
			const urlObj = new URL(url);
			const pathname = urlObj.pathname;
			return pathname.split('/').pop() || 'unknown-file.zip';
		} catch {
			return 'unknown-file.zip';
		}
	}

	/**
	 * Launch a build
	 */
	private async launchBuild(build: BlenderBuildInfo): Promise<void> {
		try {
			await this.buildManager.launchBuild(build);
		} catch (error) {
			console.error('Failed to launch build:', error);
			new Notice(`Failed to launch build: ${error.message}`);
		}
	}

	/**
	 * Create a symlink to a build
	 */
	private async symlinkBuild(build: BlenderBuildInfo): Promise<void> {
		try {
			await this.buildManager.symlinkBuild(build);
			// Refresh the view to update the UI
			this.onRefresh();
		} catch (error) {
			console.error('Failed to symlink build:', error);
			new Notice(`Failed to symlink build: ${error.message}`);
		}
	}

	/**
	 * Extract filename from URL (removes .zip extension since all builds are ZIP files)
	 */
	private getFilenameFromUrl(url: string): string {
		try {
			const urlObj = new URL(url);
			const pathname = urlObj.pathname;
			let filename = pathname.split('/').pop() || 'unknown-file';
			
			// Remove .zip extension since all Blender builds are ZIP files
			if (filename.toLowerCase().endsWith('.zip')) {
				filename = filename.slice(0, -4);
			}
			
			return filename;
		} catch {
			return 'unknown-file';
		}
	}
	/**
	 * Highlight text helper - wraps matches in <mark> tags for bold highlighting
	 */
	private highlightText(needle: string, haystack: string): string {
		if (!needle || !haystack) {
			return haystack;
		}
		
		// Escape special regex characters in the search term
		const escapedNeedle = needle.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
		const regex = new RegExp(`(${escapedNeedle})`, 'gi');
		return haystack.replace(regex, '<mark>$1</mark>');
	}
}
