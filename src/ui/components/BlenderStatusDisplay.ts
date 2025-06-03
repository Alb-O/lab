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

		// Scraping status
		this.renderScrapingStatus();
	}

	/**
	 * Render scraping status section
	 */
	private renderScrapingStatus(): void {
		if (!this.statusEl) return;

		// Get actual scraping status from build manager
		const status = this.buildManager.getScrapingStatus();

		// Create a single status line without extra containers
		const statusText = status.isActive ? status.currentTask || 'Active' : 'Idle';
		const timeText = status.lastChecked ? ` • Last checked: ${status.lastChecked.toLocaleTimeString()}` : '';
		
		const statusLine = this.statusEl.createDiv('blender-status-line');
		statusLine.createSpan({ 
			text: statusText + timeText,
			cls: status.isActive ? 'blender-status-active' : 'blender-status-idle'
		});

		// Show progress bar if active and has progress (directly in statusEl)
		if (status.isActive && status.progress > 0) {
			const progressBar = this.statusEl.createDiv('blender-progress-bar');
			progressBar.style.width = `${status.progress}%`;
		}
	}
}
