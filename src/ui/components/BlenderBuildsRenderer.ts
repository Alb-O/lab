import { ButtonComponent, setTooltip } from 'obsidian';
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
		searchFilter?: string, 
		highlightFunction?: (needle: string, haystack: string) => string
	): void {
		container.empty();
		
		if (builds.length === 0) {
			this.renderEmptyState(container);
			return;
		}

		const buildsList = container.createEl('div', { cls: 'blender-builds-list' });
		
		builds.forEach((build, index) => {
			this.createBuildItem(buildsList, build, index, searchFilter, highlightFunction);
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
		highlightFunction?: (needle: string, haystack: string) => string
	): void {
		const listItem = buildsList.createEl('div', { cls: 'blender-build-item' });
		
		// Main content area
		const contentEl = listItem.createEl('div', { cls: 'blender-build-content' });
				// First line: Version, Branch, Date, Hash
		const mainLineEl = contentEl.createEl('div', { cls: 'blender-build-main-line' });
				// Version (more prominent) - with highlighting
		const versionText = searchFilter && highlightFunction ? 
			highlightFunction(searchFilter, build.subversion) : build.subversion;
		const versionEl = mainLineEl.createEl('span', { 
			cls: 'blender-build-version'
		});
		versionEl.innerHTML = versionText;
		setTooltip(versionEl, `Version: ${build.subversion}`);

		// Branch tag - with highlighting
		const branchText = searchFilter && highlightFunction ? 
			highlightFunction(searchFilter, build.branch.toUpperCase()) : build.branch.toUpperCase();
		const branchEl = mainLineEl.createEl('span', { 
			cls: `blender-branch-tag branch-${build.branch.toLowerCase()}`
		});
		branchEl.innerHTML = branchText;
		setTooltip(branchEl, `Branch: ${build.branch}`);
		// Build hash - with highlighting
		if (build.buildHash) {
			const hashText = searchFilter && highlightFunction ? 
				highlightFunction(searchFilter, build.buildHash.substring(0, 8)) : build.buildHash.substring(0, 8);
			const hashEl = mainLineEl.createEl('span', { 
				cls: 'blender-build-hash'
			});
			hashEl.innerHTML = hashText;
			setTooltip(hashEl, `Build hash: ${build.buildHash}`);
		}

		// Date - with highlighting
		const dateText = searchFilter && highlightFunction ? 
			highlightFunction(searchFilter, build.commitTime.toLocaleDateString()) : build.commitTime.toLocaleDateString();
		const dateEl = mainLineEl.createEl('span', { 
			cls: 'blender-build-date'
		});
		dateEl.innerHTML = dateText;
		setTooltip(dateEl, `Committed: ${build.commitTime.toLocaleString()}`);

		// Second line: Filename only
		const detailsLineEl = contentEl.createEl('div', { cls: 'blender-build-details-line' });
		
		// Filename - with highlighting
		const filename = this.getFilenameFromUrl(build.link);
		const filenameText = searchFilter && highlightFunction ? 
			highlightFunction(searchFilter, filename) : filename;
		const filenameEl = detailsLineEl.createEl('span', { 
			cls: 'blender-build-filename'
		});
		filenameEl.innerHTML = filenameText;
		setTooltip(filenameEl, `File: ${filename}`);

		// Add action buttons
		const actionsEl = listItem.createEl('div', { cls: 'blender-build-actions' });
		this.addBuildActions(actionsEl, build, index);

		// Make the entire item clickable for download
		listItem.addClass('blender-build-clickable');
		listItem.addEventListener('click', async (evt) => {
			// Don't trigger download if clicking on action buttons
			if ((evt.target as HTMLElement).closest('.blender-build-actions')) {
				return;
			}
			
			evt.preventDefault();
			evt.stopPropagation();
			
			try {
				await this.downloadBuild(build);
			} catch (error) {
				console.error('Error downloading build:', error);
			}
		});
	}

	/**
	 * Add action buttons for a build item
	 */
	private addBuildActions(actionsEl: HTMLElement, build: BlenderBuildInfo, index: number): void {
		// Download button
		const downloadBtn = new ButtonComponent(actionsEl)
			.setIcon('download')
			.setTooltip('Download this build')
			.setClass('clickable-icon')
			.setClass('blender-action-button');
		
		downloadBtn.buttonEl.addEventListener('click', (evt) => {
			evt.preventDefault();
			evt.stopPropagation();
			this.downloadBuild(build);
		});

		// Info button
		const infoBtn = new ButtonComponent(actionsEl)
			.setIcon('info')
			.setTooltip('Build information')
			.setClass('clickable-icon')
			.setClass('blender-action-button');
		
		infoBtn.buttonEl.addEventListener('click', (evt) => {
			evt.preventDefault();
			evt.stopPropagation();
			this.showBuildInfo(build);
		});

		// Copy link button
		const copyBtn = new ButtonComponent(actionsEl)
			.setIcon('copy')
			.setTooltip('Copy download link')
			.setClass('clickable-icon')
			.setClass('blender-action-button');
		
		copyBtn.buttonEl.addEventListener('click', (evt) => {
			evt.preventDefault();
			evt.stopPropagation();
			navigator.clipboard.writeText(build.link);
		});
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
		} catch (error) {
			console.error('Failed to download build:', error);
		}
	}

	/**
	 * Show build information
	 */
	private showBuildInfo(build: BlenderBuildInfo): void {
		// TODO: Implement build info modal
		console.log('Build info:', build);
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
}
