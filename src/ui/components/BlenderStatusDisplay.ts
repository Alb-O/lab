import { FetchBlenderBuilds } from '../../buildManager';
import { ScrapingStatus, DownloadProgress, BlenderBuildInfo } from '../../types';

export class BlenderStatusDisplay {
	private buildManager: FetchBlenderBuilds;
	private statusEl: HTMLElement | null = null;
	private currentActivity: string | null = null;
	private activityStartTime: Date | null = null;

	constructor(buildManager: FetchBlenderBuilds) {
		this.buildManager = buildManager;
		this.setupEventListeners();
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

		// Show current activity (download/extraction) if active, otherwise show scraping status
		if (this.currentActivity) {
			this.renderActivityStatus();
		} else {
			this.renderScrapingStatus();
		}
	}

	/**
	 * Render current activity status (download/extraction)
	 */
	private renderActivityStatus(): void {
		if (!this.statusEl || !this.currentActivity) return;

		const statusLine = this.statusEl.createDiv('blender-status-line');
		statusLine.createSpan({ 
			text: this.currentActivity,
			cls: 'blender-status-active'
		});

		// Show elapsed time for long-running activities
		if (this.activityStartTime) {
			const elapsed = Math.floor((Date.now() - this.activityStartTime.getTime()) / 1000);
			if (elapsed > 5) { // Show elapsed time after 5 seconds
				const timeText = ` (${elapsed}s)`;
				statusLine.createSpan({ 
					text: timeText,
					cls: 'blender-status-time'
				});
			}
		}
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

	/**
	 * Setup event listeners for build manager events
	 */
	private setupEventListeners(): void {
		// Download events
		this.buildManager.on('downloadStarted', (build: BlenderBuildInfo) => {
			this.setActivity(`Downloading ${build.subversion}...`);
		});

		this.buildManager.on('downloadCompleted', (build: BlenderBuildInfo) => {
			this.setActivity(`Downloaded ${build.subversion} successfully`);
			// Clear activity after 3 seconds
			setTimeout(() => this.clearActivity(), 3000);
		});

		this.buildManager.on('downloadError', (build: BlenderBuildInfo, error: any) => {
			this.setActivity(`Download failed: ${build.subversion}`);
			// Clear activity after 5 seconds for errors
			setTimeout(() => this.clearActivity(), 5000);
		});

		// Extraction events
		this.buildManager.on('extractionStarted', (archivePath: string) => {
			const fileName = archivePath.split(/[/\\]/).pop()?.replace(/\.[^/.]+$/, '') || 'build';
			this.setActivity(`Extracting ${fileName}...`);
		});

		this.buildManager.on('extractionCompleted', (archivePath: string) => {
			const fileName = archivePath.split(/[/\\]/).pop()?.replace(/\.[^/.]+$/, '') || 'build';
			this.setActivity(`Extracted ${fileName} successfully`);
			// Clear activity after 3 seconds
			setTimeout(() => this.clearActivity(), 3000);
		});

		this.buildManager.on('extractionError', (archivePath: string, error: any) => {
			const fileName = archivePath.split(/[/\\]/).pop()?.replace(/\.[^/.]+$/, '') || 'build';
			this.setActivity(`Extraction failed: ${fileName}`);
			// Clear activity after 5 seconds for errors
			setTimeout(() => this.clearActivity(), 5000);
		});

		// Build extraction events (for manual extraction)
		this.buildManager.on('buildExtracted', (build: BlenderBuildInfo, extractedPath: string) => {
			this.setActivity(`Extracted ${build.subversion} successfully`);
			// Clear activity after 3 seconds
			setTimeout(() => this.clearActivity(), 3000);
		});

		// Scraping events
		this.buildManager.on('scrapingStatus', () => {
			// Only refresh if no current download/extraction activity
			if (!this.currentActivity) {
				this.refreshStatus();
			}
		});
	}
	/**
	 * Set current activity status
	 */
	private setActivity(activity: string): void {
		this.currentActivity = activity;
		this.activityStartTime = new Date();
		this.refreshStatus();
		
		// Update elapsed time every 5 seconds for long-running activities
		const updateTimer = setInterval(() => {
			if (this.currentActivity === activity) {
				this.refreshStatus();
			} else {
				clearInterval(updateTimer);
			}
		}, 5000);
	}

	/**
	 * Clear current activity status
	 */
	private clearActivity(): void {
		this.currentActivity = null;
		this.activityStartTime = null;
		this.refreshStatus();
	}
}
