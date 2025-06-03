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

		const buildsList = container.createEl('ul', { cls: 'blender-builds-list' });
		
		builds.forEach((build, index) => {
			this.createBuildItem(buildsList, build, index);
		});
	}

	/**
	 * Create a single build item (inspired by SVN history items)
	 */
	private createBuildItem(buildsList: HTMLElement, build: BlenderBuildInfo, index: number): void {
		const listItem = buildsList.createEl('li', { cls: 'blender-build-item' });
		
		// Create main content container
		const contentEl = listItem.createEl('div', { cls: 'blender-build-info-container' });
		
		// Create header with build info
		const headerEl = contentEl.createEl('div', { cls: 'blender-build-header' });
		
		// Branch indicator
		const branchEl = headerEl.createEl('span', { 
			text: build.branch.toUpperCase(),
			cls: `blender-branch blender-branch-${build.branch.toLowerCase()}`
		});
		setTooltip(branchEl, `Branch: ${build.branch}`);

		// Subversion
		const subversionEl = headerEl.createEl('span', { 
			text: build.subversion,
			cls: 'blender-subversion'
		});
		setTooltip(subversionEl, `Subversion: ${build.subversion}`);

		// Commit time
		const dateEl = headerEl.createEl('span', { 
			text: build.commitTime.toLocaleDateString(),
			cls: 'blender-commit-date'
		});
		setTooltip(dateEl, `Committed: ${build.commitTime.toLocaleString()}`);

		// Build hash (if available)
		if (build.buildHash) {
			const hashEl = headerEl.createEl('span', { 
				text: build.buildHash.substring(0, 8),
				cls: 'blender-build-hash'
			});
			setTooltip(hashEl, `Build hash: ${build.buildHash}`);
		}

		// Create content area for additional info
		const infoEl = contentEl.createEl('div', { cls: 'blender-build-details' });
		
		// Add link preview
		const linkEl = infoEl.createEl('div', { cls: 'blender-build-link' });
		linkEl.createSpan({
			text: this.getFilenameFromUrl(build.link),
			cls: 'blender-filename'
		});

		// Add action buttons
		const actionsEl = listItem.createEl('div', { cls: 'blender-build-actions' });
		this.addBuildActions(actionsEl, build, index);

		// Make the entire item clickable for download
		listItem.addClass('clickable-build-item');
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
			.setClass('clickable-icon');
		
		downloadBtn.buttonEl.addEventListener('click', (evt) => {
			evt.preventDefault();
			evt.stopPropagation();
			this.downloadBuild(build);
		});

		// Info button
		const infoBtn = new ButtonComponent(actionsEl)
			.setIcon('info')
			.setTooltip('Build information')
			.setClass('clickable-icon');
		
		infoBtn.buttonEl.addEventListener('click', (evt) => {
			evt.preventDefault();
			evt.stopPropagation();
			this.showBuildInfo(build);
		});

		// Copy link button
		const copyBtn = new ButtonComponent(actionsEl)
			.setIcon('copy')
			.setTooltip('Copy download link')
			.setClass('clickable-icon');
		
		copyBtn.buttonEl.addEventListener('click', (evt) => {
			evt.preventDefault();
			evt.stopPropagation();
			navigator.clipboard.writeText(build.link);
		});
	}

	/**
	 * Render empty state
	 */
	private renderEmptyState(container: HTMLElement): void {
		const emptyState = container.createDiv('blender-empty-state');
		
		emptyState.createEl('div', {
			text: '📦',
			cls: 'empty-state-icon'
		});
		
		emptyState.createEl('h3', {
			text: 'No Blender builds found',
			cls: 'empty-state-title'
		});
		
		emptyState.createEl('p', {
			text: 'Click the refresh button to scrape for available builds.',
			cls: 'empty-state-description'
		});

		const refreshBtn = new ButtonComponent(emptyState)
			.setButtonText('Refresh Builds')
			.setCta()
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
