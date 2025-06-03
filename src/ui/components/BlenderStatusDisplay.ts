import { FetchBlenderBuilds } from '../../buildManager';
import { ScrapingStatus, DownloadProgress } from '../../types';

export class BlenderStatusDisplay {
	private buildManager: FetchBlenderBuilds;
	private statusEl: HTMLElement | null = null;

	constructor(buildManager: FetchBlenderBuilds) {
		this.buildManager = buildManager;
	}

	/**
	 * Render status display in container
	 */
	render(container: HTMLElement): void {
		container.empty();
		this.statusEl = container;
		this.refreshStatus();
	}

	/**
	 * Update status display
	 */
	refreshStatus(): void {
		if (!this.statusEl) return;

		this.statusEl.empty();
		this.statusEl.addClass('blender-status-container');

		// Scraping status
		this.renderScrapingStatus();
		
		// Download status
		this.renderDownloadStatus();
		
		// Builds summary
		this.renderBuildsSummary();
	}

	/**
	 * Render scraping status section
	 */
	private renderScrapingStatus(): void {
		if (!this.statusEl) return;

		const statusContainer = this.statusEl.createDiv('status-section');
		const statusHeader = statusContainer.createEl('h4', { 
			text: 'Scraping Status',
			cls: 'status-header'
		});

		const statusContent = statusContainer.createDiv('status-content');
		
		// This would be connected to real scraping status
		const status: ScrapingStatus = {
			isActive: false,
			currentTask: 'Idle',
			progress: 0,
			lastChecked: new Date()
		};

		const statusLine = statusContent.createDiv('status-line');
		statusLine.createSpan({ 
			text: status.isActive ? 'Active' : 'Idle',
			cls: status.isActive ? 'status-active' : 'status-idle'
		});

		if (status.lastChecked) {
			statusContent.createDiv({
				text: `Last checked: ${status.lastChecked.toLocaleTimeString()}`,
				cls: 'status-time'
			});
		}

		if (status.isActive && status.progress > 0) {
			const progressContainer = statusContent.createDiv('progress-container');
			const progressBar = progressContainer.createDiv('progress-bar');
			progressBar.style.width = `${status.progress}%`;
		}
	}

	/**
	 * Render download status section
	 */
	private renderDownloadStatus(): void {
		if (!this.statusEl) return;

		// Only show if there are active downloads
		const activeDownloads = this.getActiveDownloads();
		if (activeDownloads.length === 0) return;

		const downloadContainer = this.statusEl.createDiv('status-section');
		downloadContainer.createEl('h4', { 
			text: 'Downloads',
			cls: 'status-header'
		});

		const downloadContent = downloadContainer.createDiv('status-content');
		
		activeDownloads.forEach(download => {
			const downloadItem = downloadContent.createDiv('download-item');
			
			downloadItem.createSpan({
				text: download.filename || 'Unknown file',
				cls: 'download-filename'
			});

			const progressContainer = downloadItem.createDiv('progress-container');
			const progressBar = progressContainer.createDiv('progress-bar');
			progressBar.style.width = `${download.percentage}%`;
			
			const progressText = progressContainer.createSpan({
				text: `${download.percentage.toFixed(1)}%`,
				cls: 'progress-text'
			});

			if (download.speed) {
				downloadItem.createSpan({
					text: this.formatSpeed(download.speed),
					cls: 'download-speed'
				});
			}
		});
	}

	/**
	 * Render builds summary section
	 */
	private renderBuildsSummary(): void {
		if (!this.statusEl) return;

		const builds = this.buildManager.getCachedBuilds();
		if (builds.length === 0) return;

		const summaryContainer = this.statusEl.createDiv('status-section');
		summaryContainer.createEl('h4', { 
			text: 'Builds Summary',
			cls: 'status-header'
		});

		const summaryContent = summaryContainer.createDiv('status-content');
		
		// Count builds by branch
		const buildsByBranch = builds.reduce((acc, build) => {
			acc[build.branch] = (acc[build.branch] || 0) + 1;
			return acc;
		}, {} as Record<string, number>);

		Object.entries(buildsByBranch).forEach(([branch, count]) => {
			const branchLine = summaryContent.createDiv('summary-line');
			branchLine.createSpan({
				text: branch.charAt(0).toUpperCase() + branch.slice(1),
				cls: 'branch-name'
			});
			branchLine.createSpan({
				text: `${count} builds`,
				cls: 'build-count'
			});
		});

		// Total count
		const totalLine = summaryContent.createDiv('summary-line total');
		totalLine.createSpan({
			text: 'Total',
			cls: 'branch-name'
		});
		totalLine.createSpan({
			text: `${builds.length} builds`,
			cls: 'build-count'
		});
	}

	/**
	 * Get active downloads (placeholder)
	 */
	private getActiveDownloads(): Array<DownloadProgress & { filename?: string }> {
		// This would be connected to real download status
		return [];
	}

	/**
	 * Format download speed
	 */
	private formatSpeed(bytesPerSecond: number): string {
		const units = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
		let size = bytesPerSecond;
		let unitIndex = 0;

		while (size >= 1024 && unitIndex < units.length - 1) {
			size /= 1024;
			unitIndex++;
		}

		return `${size.toFixed(1)} ${units[unitIndex]}`;
	}
}
