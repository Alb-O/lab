import { ButtonComponent, setTooltip, Notice, setIcon, Platform } from 'obsidian';
import { BlenderBuildInfo } from '../../types';
import { FetchBlenderBuilds } from '../../buildManager';
import type BlenderBuildManagerPlugin from '../../main';
import { ConfirmDeleteBuildModal } from './ConfirmDeleteBuildModal';
import * as path from 'path';

export class BlenderBuildsRenderer {
	private plugin: BlenderBuildManagerPlugin;
	private buildManager: FetchBlenderBuilds;
	private onRefresh: () => void;

	constructor(
		plugin: BlenderBuildManagerPlugin,
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
		searchFilter?: string,
		pinSymlinkedBuild?: boolean,
		allBuilds?: BlenderBuildInfo[]
	): void {
		container.empty();

		if (builds.length === 0 && !pinSymlinkedBuild) {
			this.renderEmptyState(container);
			return;
		}

		// Check if we need to show pinned build
		let pinnedBuild: BlenderBuildInfo | null = null;
		let remainingBuilds = builds;
		if (pinSymlinkedBuild) {
			// Find the currently symlinked build from ALL builds (not just filtered ones)
			const buildsToSearchIn = allBuilds || builds;
			pinnedBuild = this.findSymlinkedBuild(buildsToSearchIn);
			if (pinnedBuild) {
				// Remove the pinned build from the main list if it exists there
				// Use multiple fields for more accurate matching to avoid confusion between builds
				remainingBuilds = builds.filter(build =>
					!(build.link === pinnedBuild!.link &&
						build.subversion === pinnedBuild!.subversion &&
						build.buildHash === pinnedBuild!.buildHash &&
						build.commitTime.getTime() === pinnedBuild!.commitTime.getTime())
				);
			}
		}
		// Render pinned build container if needed
		if (pinSymlinkedBuild) {
			if (pinnedBuild) {
				this.renderPinnedBuildContainer(container, pinnedBuild, searchFilter);
			} else {
				this.renderEmptyPinnedContainer(container);
			}
		}
		// Render main builds list
		const buildsList = container.createEl('div', { cls: 'blender-builds-list' });

		if (remainingBuilds.length === 0 && !pinnedBuild) {
			// Only show empty state if there are no builds AND no pinned build
			this.renderEmptyState(buildsList);
		} else {
			remainingBuilds.forEach((build, index) => {
				this.createBuildItem(buildsList, build, index, searchFilter);
			});
		}
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

		// Add orphaned class if this is an orphaned build
		if (build.isOrphanedInstall) {
			listItem.addClass('orphaned');
		}
		// Main content area
		const contentEl = listItem.createEl('div', { cls: 'blender-build-content' });
		// First line: Version, Branch, Date, Hash
		const mainLineEl = contentEl.createEl('div', { cls: 'blender-build-main-line' });

		// Version (more prominent) - with highlighting
		const versionEl = mainLineEl.createEl('span', {
			cls: 'blender-build-version'
		});
		this.setTextWithHighlight(versionEl, build.subversion, searchFilter);
		setTooltip(versionEl, `Version: ${build.subversion}`);
		// Branch tag - with highlighting
		const branchEl = mainLineEl.createEl('span', {
			cls: `blender-branch-tag branch-${build.branch.toLowerCase()}`
		});
		this.setTextWithHighlight(branchEl, build.branch.toUpperCase(), searchFilter);
		setTooltip(branchEl, `Branch: ${build.branch}`);
		// Build hash - with highlighting
		if (build.buildHash) {
			const hashEl = mainLineEl.createEl('span', {
				cls: 'blender-build-hash'
			});
			this.setTextWithHighlight(hashEl, build.buildHash.substring(0, 8), searchFilter);
			setTooltip(hashEl, `Build hash: ${build.buildHash}`);
		}
		// Date - with highlighting
		const dateEl = mainLineEl.createEl('span', {
			cls: 'blender-build-date'
		});
		this.setTextWithHighlight(dateEl, build.commitTime.toLocaleDateString(), searchFilter);
		setTooltip(dateEl, `Committed: ${build.commitTime.toLocaleString()}`);

		// Second line: Filename only
		const detailsLineEl = contentEl.createEl('div', { cls: 'blender-build-details-line' });
		// Filename - with highlighting
		const filename = this.getFilenameFromUrl(build.link);
		const filenameEl = detailsLineEl.createEl('span', {
			cls: 'blender-build-filename'
		});
		this.setTextWithHighlight(filenameEl, filename, searchFilter);
		setTooltip(filenameEl, `File: ${filename}`);
		// Check if build is installed to determine clickable behavior
		const installStatus = this.buildManager.isBuildInstalled(build);
		const isInstalled = installStatus.downloaded || installStatus.extracted;
		const isSymlinked = installStatus.extracted && this.isBuildSymlinked(build);

		// Add action buttons (pass install status to avoid duplicate calls)
		const actionsEl = listItem.createEl('div', { cls: 'blender-build-actions' });
		this.addBuildActions(actionsEl, build, index, installStatus);

		// Make the entire item clickable for download only if not installed
		if (isInstalled) {
			// For installed builds, add a different visual style
			listItem.addClass('blender-build-installed');

			// Add special styling for symlinked build
			if (isSymlinked) {
				listItem.addClass('blender-build-symlinked');
			}
		}
	}

	/**
	 * Add action buttons for a build item
	 */
	private addBuildActions(actionsEl: HTMLElement, build: BlenderBuildInfo, index: number, installStatus: { downloaded: boolean; extracted: boolean }): void {
		const isInstalled = installStatus.downloaded || installStatus.extracted;
		if (isInstalled) {
			// For installed builds: Launch button (first), Show in Explorer, Extract button (if needed), Symlink button, Trash button (last)

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

			// Show in Explorer button - show for all installed builds
			const explorerBtn = new ButtonComponent(actionsEl)
				.setIcon('folder-open')
				.setTooltip('Show in system explorer')
				.setClass('clickable-icon')
				.setClass('blender-action-button')
				.setClass('blender-explorer-button');
			explorerBtn.buttonEl.addEventListener('click', async (evt) => {
				evt.preventDefault();
				evt.stopPropagation();
				try {
					await this.showBuildInExplorer(build, installStatus);
				} catch (error) {
					console.error('Failed to show build in explorer:', error);
					new Notice(`Failed to show ${build.subversion} in explorer: ${error.message}`);
				}
			});
			// Extract button - show only if downloaded but not extracted and auto-extract is off
			if (installStatus.downloaded && !installStatus.extracted && !this.plugin.settings.autoExtract) {
				const extractBtn = new ButtonComponent(actionsEl)
					.setIcon('archive')
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
				const isCurrentlySymlinked = this.isBuildSymlinked(build);

				const symlinkBtn = new ButtonComponent(actionsEl)
					.setIcon('link')
					.setTooltip(isCurrentlySymlinked ? 'Remove symlink from this build' : 'Create symlink to this build')
					.setClass('clickable-icon')
					.setClass('blender-action-button')
					.setClass('blender-symlink-button');

				if (isCurrentlySymlinked) {
					symlinkBtn.buttonEl.addClass('is-active');
				}

				symlinkBtn.buttonEl.addEventListener('click', async (evt) => {
					evt.preventDefault();
					evt.stopPropagation();
					try {
						if (isCurrentlySymlinked) {
							await this.unsymlinkBuild(build);
						} else {
							await this.symlinkBuild(build);
						}
					} catch (error) {
						console.error('Failed to symlink/unsymlink build:', error);
						new Notice(`Failed to ${isCurrentlySymlinked ? 'unsymlink' : 'symlink'} ${build.subversion}: ${error.message}`);
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

				// Show confirmation modal before deleting
				const modal = new ConfirmDeleteBuildModal(
					this.plugin.app,
					build,
					async () => {
						try {
							await this.deleteBuild(build);
						} catch (error) {
							console.error('Failed to delete build:', error);
							new Notice(`Failed to delete ${build.subversion}: ${error.message}`);
						}
					}
				);
				modal.open();
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
	}

	/**
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
	 * Find the currently symlinked build from a list of builds
	 */
	private findSymlinkedBuild(builds: BlenderBuildInfo[]): BlenderBuildInfo | null {
		// Check if there's a symlink and find the matching build
		try {
			const buildsRootPath = this.buildManager.getBuildsPath();
			const path = require('path');
			const symlinkPath = path.join(buildsRootPath, 'bl_symlink');
			const fs = require('fs');

			if (!fs.existsSync(symlinkPath)) {
				return null;
			}

			// Read the symlink target
			const symlinkTarget = fs.readlinkSync(symlinkPath);

			// Find the build that matches the symlink target
			for (const build of builds) {
				const installStatus = this.buildManager.isBuildInstalled(build);
				if (installStatus.extracted) {
					const extractPath = this.buildManager.getExtractsPathForBuild(build);
					if (symlinkTarget.includes(extractPath) || extractPath.includes(symlinkTarget)) {
						return build;
					}
				}
			}
		} catch (error) {
			console.error('Error finding symlinked build:', error);
		}

		return null;
	}

	/**
	 * Check if a specific build is currently symlinked
	 */
	private isBuildSymlinked(build: BlenderBuildInfo): boolean {
		try {
			const buildsRootPath = this.buildManager.getBuildsPath();
			const path = require('path');
			const symlinkPath = path.join(buildsRootPath, 'bl_symlink');
			const fs = require('fs');

			if (!fs.existsSync(symlinkPath)) {
				return false;
			}

			// Read the symlink target
			const symlinkTarget = fs.readlinkSync(symlinkPath);

			// Check if this build's extract path matches the symlink target
			const installStatus = this.buildManager.isBuildInstalled(build);
			if (installStatus.extracted) {
				const extractPath = this.buildManager.getExtractsPathForBuild(build);
				return symlinkTarget.includes(extractPath) || extractPath.includes(symlinkTarget);
			}
		} catch (error) {
			console.error('Error checking if build is symlinked:', error);
		}

		return false;
	}

	/**
	 * Remove symlink (unlink a build)
	 */
	private async unsymlinkBuild(build: BlenderBuildInfo): Promise<void> {
		try {
			const buildsRootPath = this.buildManager.getBuildsPath();
			const path = require('path');
			const symlinkPath = path.join(buildsRootPath, 'bl_symlink');
			const fs = require('fs');
			// Remove existing symlink if it exists (including broken symlinks)
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
					new Notice(`Removed symlink for ${build.subversion}`);

					// Emit event similar to buildManager (for consistency)
					this.buildManager.emit('buildUnsymlinked', build, symlinkPath);

					// Refresh the view to update the UI
					this.onRefresh();
				} else {
					new Notice(`bl_symlink exists but is not a symlink - cannot remove`);
				}
			} catch (error: any) {
				// If lstatSync throws ENOENT, the path doesn't exist
				if (error.code === 'ENOENT') {
					new Notice(`No symlink found for ${build.subversion}`);
				} else {
					throw error;
				}
			}
		} catch (error) {
			console.error('Failed to remove symlink:', error);
			throw error;
		}
	}

	/**
	 * Show build in system explorer
	 */
	private async showBuildInExplorer(build: BlenderBuildInfo, installStatus: { downloaded: boolean; extracted: boolean }): Promise<void> {
		try {
			const { exec } = require('child_process');
			const fs = require('fs');
			let pathToOpen: string;

			// Handle orphaned builds differently
			if (build.isOrphanedInstall) {
				if (installStatus.extracted && build.extractedPath) {
					pathToOpen = build.extractedPath;
				} else if (installStatus.downloaded && build.archivePath) {
					pathToOpen = path.dirname(build.archivePath);
				} else {
					new Notice('Build paths not found');
					return;
				}
			} else {
				// Handle regular builds
				if (installStatus.extracted) {
					// Show the extracted build folder
					pathToOpen = this.buildManager.getExtractsPathForBuild(build);
				} else if (installStatus.downloaded) {
					// Show the downloads folder containing the zip file
					pathToOpen = this.buildManager.getDownloadsPath();
				} else {
					// This shouldn't happen since the button is only shown for installed builds
					new Notice('Build is not installed');
					return;
				}
			}

			// Check if the path exists
			if (!fs.existsSync(pathToOpen)) {
				new Notice(`Path does not exist: ${pathToOpen}`);
				return;
			}
			// Open folder in system file manager
			if (Platform.isWin) {
				exec(`explorer "${pathToOpen}"`);
			} else if (Platform.isMacOS) {
				exec(`open "${pathToOpen}"`);
			} else {
				exec(`xdg-open "${pathToOpen}"`);
			}
		} catch (error) {
			console.error('Failed to show build in explorer:', error);
			new Notice(`Failed to show build in explorer: ${error.message}`);
		}
	}

	/**
	 * Render the pinned build container
	 */
	private renderPinnedBuildContainer(container: HTMLElement, pinnedBuild: BlenderBuildInfo, searchFilter?: string): void {
		const pinnedContainer = container.createEl('div', { cls: 'blender-pinned-builds-container' });

		// Add header
		const header = pinnedContainer.createEl('div', { cls: 'blender-pinned-header' });
		const titleContainer = header.createEl('div', { cls: 'blender-pinned-title' });

		// Add pin icon using built-in Obsidian icon
		const calloutIcon = titleContainer.createDiv({ cls: "blender-pinned-icon" });
		setIcon(calloutIcon, "pin");

		// Add title text
		titleContainer.createEl('span', { text: 'Pinned build' });
		// Add pinned build item with special styling
		const pinnedList = pinnedContainer.createEl('div', { cls: 'blender-pinned-builds-list' });
		this.createBuildItem(pinnedList, pinnedBuild, 0, searchFilter);

		// Add special class to the pinned build item
		const pinnedBuildItem = pinnedList.querySelector('.blender-build-item');
		if (pinnedBuildItem) {
			pinnedBuildItem.addClass('blender-build-pinned');
		}
	}

	/**
	 * Render empty pinned build container when pin is enabled but no symlinked build exists
	 */
	private renderEmptyPinnedContainer(container: HTMLElement): void {
		const pinnedContainer = container.createEl('div', { cls: 'blender-pinned-builds-container blender-pinned-empty' });

		// Add header
		const header = pinnedContainer.createEl('div', { cls: 'blender-pinned-header' });
		const titleContainer = header.createEl('div', { cls: 'blender-pinned-title' });

		// Add pin icon using built-in Obsidian icon
		const calloutIcon = titleContainer.createDiv({ cls: "blender-pinned-icon" });
		setIcon(calloutIcon, "pin");

		// Add title text
		titleContainer.createEl('span', { text: 'Pinned build' });

		// Add empty message
		const emptyMessage = pinnedContainer.createEl('div', {
			cls: 'blender-pinned-empty-message',
			text: 'No build is currently symlinked. Create a symlink to pin a build here.'
		});
	}

	/**
	 * Safely set text content with optional highlighting
	 */
	private setTextWithHighlight(element: HTMLElement, text: string, searchFilter?: string): void {
		if (!searchFilter) {
			element.setText(text);
			return;
		}

		// Clear the element
		element.empty();

		// Split text by the search filter to create highlighted portions
		const escapedFilter = searchFilter.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
		const regex = new RegExp(`(${escapedFilter})`, 'gi');
		const parts = text.split(regex);

		parts.forEach((part, index) => {
			if (part.toLowerCase() === searchFilter.toLowerCase()) {
				// Create highlighted part
				element.createEl('mark', { text: part });
			} else if (part) {
				// Create regular text node
				element.appendText(part);
			}
		});
	}
}
