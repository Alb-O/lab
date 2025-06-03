import { BlenderBuildInfo, DownloadProgress, ExtractionProgress, ScrapingStatus } from './types';
import { BlenderPluginSettings } from './settings';
import { FetchBlenderBuilds } from './buildManager';
import { Modal, App, Setting, Notice, ButtonComponent, ProgressBarComponent } from 'obsidian';

export class BlenderBuildsModal extends Modal {
	private buildManager: FetchBlenderBuilds;
	private builds: BlenderBuildInfo[] = [];
	private filteredBuilds: BlenderBuildInfo[] = [];
	private currentFilter: string = '';
	private currentBranch: string = 'all';

	constructor(app: App, buildManager: FetchBlenderBuilds) {
		super(app);
		this.buildManager = buildManager;
		this.builds = buildManager.getCachedBuilds();
		this.filteredBuilds = [...this.builds];
	}

	onOpen() {
		const { contentEl } = this;
		contentEl.empty();
		contentEl.addClass('blender-builds-modal');

		this.createHeader();
		this.createFilters();
		this.createBuildsList();
		this.createFooter();

		// Set up event listeners
		this.buildManager.on('buildsUpdated', this.onBuildsUpdated.bind(this));
		this.buildManager.on('scrapingStatus', this.onScrapingStatus.bind(this));
		this.buildManager.on('downloadStarted', this.onDownloadStarted.bind(this));
		this.buildManager.on('downloadCompleted', this.onDownloadCompleted.bind(this));
	}

	onClose() {
		const { contentEl } = this;
		contentEl.empty();
		
		// Remove event listeners
		this.buildManager.removeAllListeners();
	}

	private createHeader(): void {
		const header = this.contentEl.createDiv('blender-builds-header');
		header.createEl('h2', { text: 'Blender Builds Manager' });
		
		const actions = header.createDiv('blender-builds-actions');
		
		new ButtonComponent(actions)
			.setButtonText('Refresh Builds')
			.setTooltip('Scrape for new builds')
			.onClick(() => this.refreshBuilds());

		new ButtonComponent(actions)
			.setButtonText('Settings')
			.setTooltip('Open plugin settings')
			.onClick(() => this.openSettings());
	}

	private createFilters(): void {
		const filters = this.contentEl.createDiv('blender-builds-filters');
		
		// Search filter
		const searchContainer = filters.createDiv('search-container');
		searchContainer.createEl('label', { text: 'Search:' });
		const searchInput = searchContainer.createEl('input', {
			type: 'text',
			placeholder: 'Filter builds...'
		});
		searchInput.addEventListener('input', (e) => {
			this.currentFilter = (e.target as HTMLInputElement).value;
			this.filterBuilds();
		});

		// Branch filter
		const branchContainer = filters.createDiv('branch-container');
		branchContainer.createEl('label', { text: 'Branch:' });
		const branchSelect = branchContainer.createEl('select');
		
		const branches = ['all', ...new Set(this.builds.map(b => b.branch))];
		branches.forEach(branch => {
			const option = branchSelect.createEl('option', {
				value: branch,
				text: branch.charAt(0).toUpperCase() + branch.slice(1)
			});
			if (branch === this.currentBranch) {
				option.selected = true;
			}
		});

		branchSelect.addEventListener('change', (e) => {
			this.currentBranch = (e.target as HTMLSelectElement).value;
			this.filterBuilds();
		});
	}

	private createBuildsList(): void {
		const listContainer = this.contentEl.createDiv('blender-builds-list-container');
		
		const listHeader = listContainer.createDiv('blender-builds-list-header');
		listHeader.createEl('span', { text: 'Version', cls: 'column-version' });
		listHeader.createEl('span', { text: 'Branch', cls: 'column-branch' });
		listHeader.createEl('span', { text: 'Date', cls: 'column-date' });
		listHeader.createEl('span', { text: 'Actions', cls: 'column-actions' });

		const list = listContainer.createDiv('blender-builds-list');
		this.updateBuildsList(list);
	}

	private updateBuildsList(listEl?: HTMLElement): void {
		const list = listEl || this.contentEl.querySelector('.blender-builds-list') as HTMLElement;
		if (!list) return;

		list.empty();

		if (this.filteredBuilds.length === 0) {
			const emptyState = list.createDiv('empty-state');
			emptyState.createEl('p', { text: 'No builds found matching your criteria.' });
			emptyState.createEl('p', { text: 'Try refreshing or adjusting your filters.' });
			return;
		}

		this.filteredBuilds.forEach(build => {
			const item = list.createDiv('blender-build-item');
			
			// Version info
			const versionEl = item.createDiv('build-version');
			versionEl.createEl('div', { text: build.subversion, cls: 'version-text' });
			if (build.buildHash) {
				versionEl.createEl('div', { text: build.buildHash.substring(0, 8), cls: 'build-hash' });
			}

			// Branch
			const branchEl = item.createDiv('build-branch');
			branchEl.createEl('span', { 
				text: build.branch,
				cls: `branch-tag branch-${build.branch}`
			});

			// Date
			const dateEl = item.createDiv('build-date');
			dateEl.createEl('div', { 
				text: build.commitTime.toLocaleDateString(),
				cls: 'date-text'
			});
			dateEl.createEl('div', { 
				text: build.commitTime.toLocaleTimeString(),
				cls: 'time-text'
			});

			// Actions
			const actionsEl = item.createDiv('build-actions');
			
			new ButtonComponent(actionsEl)
				.setButtonText('Download')
				.setClass('mod-cta')
				.onClick(() => this.downloadBuild(build));

			new ButtonComponent(actionsEl)
				.setButtonText('Info')
				.onClick(() => this.showBuildInfo(build));
		});
	}

	private createFooter(): void {
		const footer = this.contentEl.createDiv('blender-builds-footer');
		
		const stats = footer.createDiv('blender-builds-stats');
		stats.createEl('span', { text: `${this.filteredBuilds.length} builds shown` });
		
		const lastUpdate = this.buildManager.getScrapingStatus().lastChecked;
		if (lastUpdate) {
			stats.createEl('span', { 
				text: `Last updated: ${lastUpdate.toLocaleString()}`,
				cls: 'last-update'
			});
		}
	}

	private filterBuilds(): void {
		this.filteredBuilds = this.builds.filter(build => {
			const matchesSearch = !this.currentFilter || 
				build.subversion.toLowerCase().includes(this.currentFilter.toLowerCase()) ||
				build.branch.toLowerCase().includes(this.currentFilter.toLowerCase()) ||
				(build.buildHash && build.buildHash.toLowerCase().includes(this.currentFilter.toLowerCase()));

			const matchesBranch = this.currentBranch === 'all' || build.branch === this.currentBranch;

			return matchesSearch && matchesBranch;
		});

		this.updateBuildsList();
		this.updateFooter();
	}

	private updateFooter(): void {
		const stats = this.contentEl.querySelector('.blender-builds-stats');
		if (stats) {
			stats.empty();
			stats.createEl('span', { text: `${this.filteredBuilds.length} of ${this.builds.length} builds shown` });
			
			const lastUpdate = this.buildManager.getScrapingStatus().lastChecked;
			if (lastUpdate) {
				stats.createEl('span', { 
					text: `Last updated: ${lastUpdate.toLocaleString()}`,
					cls: 'last-update'
				});
			}
		}
	}

	private async refreshBuilds(): Promise<void> {
		try {
			const refreshButton = this.contentEl.querySelector('button') as HTMLButtonElement;
			if (refreshButton) {
				refreshButton.textContent = 'Refreshing...';
				refreshButton.disabled = true;
			}

			await this.buildManager.refreshBuilds();
		} catch (error) {
			new Notice(`Failed to refresh builds: ${error.message}`);
		}
	}
    private async downloadBuild(build: BlenderBuildInfo): Promise<void> {
		try {
			const progressModal = new DownloadProgressModal(this.app, build);
			progressModal.open();

			// Download the build
			const filePath = await this.buildManager.downloadBuild(build, (progress: DownloadProgress) => {
				progressModal.updateProgress(progress);
			});

			// Auto-extract if enabled
			const settings = this.buildManager.getSettings();
			if (settings.autoExtract) {
				progressModal.updateStatus('Extracting...');
				
				await this.buildManager.extractBuild(filePath, build, (progress: ExtractionProgress) => {
					// Update progress modal with extraction progress
					const downloadProgress: DownloadProgress = {
						downloaded: progress.extractedFiles,
						total: progress.totalFiles,
						percentage: progress.percentage,
						speed: 0,
						status: 'extracting'
					};
					progressModal.updateProgress(downloadProgress);
				});
				
				// Always clean up archive and empty downloads folder after extraction
				await this.buildManager.cleanupAfterExtraction(filePath);
			}

			progressModal.close();
			const message = settings.autoExtract 
				? `Downloaded and extracted: ${build.subversion}`
				: `Downloaded: ${build.subversion}`;
			new Notice(message);
		} catch (error) {
			new Notice(`Download failed: ${error.message}`);
		}
	}

	private showBuildInfo(build: BlenderBuildInfo): void {
		const infoModal = new BuildInfoModal(this.app, build);
		infoModal.open();
	}

	private openSettings(): void {
		// This would typically open the plugin settings tab
		// For now, we'll show a notice
		new Notice('Settings functionality would be implemented here');
	}

	// Event handlers
	private onBuildsUpdated(builds: BlenderBuildInfo[]): void {
		this.builds = builds;
		this.filteredBuilds = [...builds];
		this.filterBuilds();
		
		// Update refresh button
		const refreshButton = this.contentEl.querySelector('button') as HTMLButtonElement;
		if (refreshButton) {
			refreshButton.textContent = 'Refresh Builds';
			refreshButton.disabled = false;
		}
	}

	private onScrapingStatus(status: ScrapingStatus): void {
		// Update UI to show scraping progress
		const refreshButton = this.contentEl.querySelector('button') as HTMLButtonElement;
		if (refreshButton) {
			if (status.isActive) {
				refreshButton.textContent = status.currentTask;
				refreshButton.disabled = true;
			} else {
				refreshButton.textContent = 'Refresh Builds';
				refreshButton.disabled = false;
			}
		}
	}

	private onDownloadStarted(build: BlenderBuildInfo, filePath: string): void {
		// Update UI to show download started
		console.log(`Download started: ${build.subversion}`);
	}

	private onDownloadCompleted(build: BlenderBuildInfo, filePath: string): void {
		// Update UI to show download completed
		console.log(`Download completed: ${build.subversion}`);
	}
}

export class DownloadProgressModal extends Modal {
	private build: BlenderBuildInfo;
	private statusEl: HTMLElement;
	private animationInterval: number | null = null;
	private dotCount = 1;

	constructor(app: App, build: BlenderBuildInfo) {
		super(app);
		this.build = build;
	}

	onOpen() {
		const { contentEl } = this;
		contentEl.empty();
		contentEl.addClass('download-progress-modal');

		const header = contentEl.createEl('h3', { text: `Downloading ${this.build.subversion}` });
		
		this.statusEl = contentEl.createDiv('download-status');
		this.statusEl.textContent = 'Starting download...';
		
		// Start the animated dots
		this.startAnimation();
	}

	onClose() {
		this.stopAnimation();
	}

	private startAnimation() {
		this.animationInterval = window.setInterval(() => {
			if (this.statusEl && this.statusEl.textContent?.includes('Downloading')) {
				const dots = '.'.repeat(this.dotCount);
				this.statusEl.textContent = `Downloading${dots}`;
				this.dotCount = this.dotCount === 3 ? 1 : this.dotCount + 1;
			}
		}, 500);
	}

	private stopAnimation() {
		if (this.animationInterval) {
			clearInterval(this.animationInterval);
			this.animationInterval = null;
		}
	}
	updateProgress(progress: DownloadProgress): void {
		if (!this.statusEl) return;

		if (progress.status === 'downloading') {
			// Animation is handled by startAnimation()
			return;
		} else if (progress.status === 'extracting') {
			this.stopAnimation();
			this.statusEl.textContent = `Extracting... ${progress.downloaded} / ${progress.total} files`;
		} else if (progress.status === 'completed') {
			this.stopAnimation();
			this.statusEl.textContent = 'Completed!';
			setTimeout(() => this.close(), 1000);
		} else if (progress.status === 'error') {
			this.stopAnimation();
			this.statusEl.textContent = progress.error || 'Download failed';
			setTimeout(() => this.close(), 2000);
		} else {
			this.stopAnimation();
			this.statusEl.textContent = progress.status || 'Processing...';
		}
	}

	updateStatus(status: string): void {
		if (this.statusEl) {
			this.statusEl.textContent = status;
		}
	}

	private formatBytes(bytes: number): string {
		if (bytes === 0) return '0 Bytes';
		const k = 1024;
		const sizes = ['Bytes', 'KB', 'MB', 'GB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
	}
}

export class BuildInfoModal extends Modal {
	private build: BlenderBuildInfo;

	constructor(app: App, build: BlenderBuildInfo) {
		super(app);
		this.build = build;
	}

	onOpen() {
		const { contentEl } = this;
		contentEl.empty();
		contentEl.addClass('build-info-modal');

		contentEl.createEl('h3', { text: `Build Information` });

		const infoList = contentEl.createDiv('build-info-list');

		this.addInfoItem(infoList, 'Version', this.build.subversion);
		this.addInfoItem(infoList, 'Branch', this.build.branch);
		this.addInfoItem(infoList, 'Commit Date', this.build.commitTime.toLocaleString());
		
		if (this.build.buildHash) {
			this.addInfoItem(infoList, 'Build Hash', this.build.buildHash);
		}

		this.addInfoItem(infoList, 'Download URL', this.build.link);

		if (this.build.customExecutable) {
			this.addInfoItem(infoList, 'Executable Path', this.build.customExecutable);
		}

		const actions = contentEl.createDiv('build-info-actions');
		
		new ButtonComponent(actions)
			.setButtonText('Copy Download URL')
			.onClick(() => {
				navigator.clipboard.writeText(this.build.link);
				new Notice('Download URL copied to clipboard');
			});

		new ButtonComponent(actions)
			.setButtonText('Close')
			.onClick(() => this.close());
	}

	private addInfoItem(container: HTMLElement, label: string, value: string): void {
		const item = container.createDiv('info-item');
		item.createEl('strong', { text: `${label}: ` });
		item.createEl('span', { text: value });
	}
}
