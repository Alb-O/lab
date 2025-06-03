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
	renderBuilds(container: HTMLElement, builds: BlenderBuildInfo[]): void {
		container.empty();
		
		if (builds.length === 0) {
			this.renderEmptyState(container);
			return;
		}

		const buildsList = container.createEl('div', { cls: 'blender-builds-list' });
		
		builds.forEach((build, index) => {
			this.createBuildItem(buildsList, build, index);
		});
	}	/**
	 * Create a single build item - compact two-line design
	 */
	private createBuildItem(buildsList: HTMLElement, build: BlenderBuildInfo, index: number): void {
		const listItem = buildsList.createEl('div', { cls: 'blender-build-item' });
		
		// Main content area
		const contentEl = listItem.createEl('div', { cls: 'blender-build-content' });
		
		// First line: Version, Branch, Date
		const mainLineEl = contentEl.createEl('div', { cls: 'blender-build-main-line' });
		
		// Version (more prominent)
		const versionEl = mainLineEl.createEl('span', { 
			text: build.subversion,
			cls: 'blender-build-version'
		});
		setTooltip(versionEl, `Version: ${build.subversion}`);

		// Branch tag
		const branchEl = mainLineEl.createEl('span', { 
			text: build.branch.toUpperCase(),
			cls: `blender-branch-tag branch-${build.branch.toLowerCase()}`
		});
		setTooltip(branchEl, `Branch: ${build.branch}`);

		// Date
		const dateEl = mainLineEl.createEl('span', { 
			text: build.commitTime.toLocaleDateString(),
			cls: 'blender-build-date'
		});
		setTooltip(dateEl, `Committed: ${build.commitTime.toLocaleString()}`);

		// Second line: Build hash and filename
		const detailsLineEl = contentEl.createEl('div', { cls: 'blender-build-details-line' });
		
		if (build.buildHash) {
			const hashEl = detailsLineEl.createEl('span', { 
				text: build.buildHash.substring(0, 8),
				cls: 'blender-build-hash'
			});
			setTooltip(hashEl, `Build hash: ${build.buildHash}`);
		}

		// Filename
		const filenameEl = detailsLineEl.createEl('span', { 
			text: this.getFilenameFromUrl(build.link),
			cls: 'blender-build-filename'
		});
		setTooltip(filenameEl, `File: ${this.getFilenameFromUrl(build.link)}`);

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
	 * Render empty state using native Obsidian setting style
	 */
	private renderEmptyState(container: HTMLElement): void {
		// Create a settings-style container
		const settingItem = container.createEl('div', { cls: 'setting-item' });
		
		// Info section with title and description
		const settingInfo = settingItem.createEl('div', { cls: 'setting-item-info' });
		settingInfo.createEl('div', { 
			text: 'Check for available Blender builds',
			cls: 'setting-item-name'
		});
		settingInfo.createEl('div', { 
			text: 'Refresh the available Blender builds from the official download servers.',
			cls: 'setting-item-description'
		});
		
		// Control section with the refresh button
		const settingControl = settingItem.createEl('div', { cls: 'setting-item-control' });
		new ButtonComponent(settingControl)
			.setButtonText('Refresh now')
			.setClass('mod-cta')
			.onClick(() => this.onRefresh());
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
	 * Extract filename from URL
	 */
	private getFilenameFromUrl(url: string): string {
		try {
			const urlObj = new URL(url);
			const pathname = urlObj.pathname;
			return pathname.split('/').pop() || 'unknown-file';
		} catch {
			return 'unknown-file';
		}
	}
}
