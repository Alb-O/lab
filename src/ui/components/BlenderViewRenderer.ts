import { BlenderBuildInfo } from '../../types';
import { FetchBlenderBuilds } from '../../buildManager';
import type FetchBlenderBuildsPlugin from '../../main';
import { 
	BlenderToolbar, 
	BlenderStatusDisplay, 
	BlenderBuildsRenderer, 
	BlenderViewLayoutManager 
} from '.';

/**
 * Main renderer component that coordinates all rendering logic for the BlenderBuildsView
 * Inspired by SVNViewRenderer from obsidian-subversion
 */
export class BlenderViewRenderer {
	private plugin: FetchBlenderBuildsPlugin;
	private buildManager: FetchBlenderBuilds;
	
	// Component managers
	private layoutManager: BlenderViewLayoutManager;
	
	// Component instances
	private toolbar: BlenderToolbar;
	private statusDisplay: BlenderStatusDisplay;
	private buildsRenderer: BlenderBuildsRenderer;
	
	// State handling
	private isInitialized = false;
	private isRefreshing = false;
	private cachedBuilds: BlenderBuildInfo[] = [];
	private currentFilter: string = '';
	private currentBranch: string = 'all';

	constructor(
		plugin: FetchBlenderBuildsPlugin,
		buildManager: FetchBlenderBuilds,
		containerEl: HTMLElement
	) {
		this.plugin = plugin;
		this.buildManager = buildManager;
		
		// Initialize layout manager
		this.layoutManager = new BlenderViewLayoutManager(containerEl);
				// Initialize component instances
		this.toolbar = new BlenderToolbar(
			this.plugin,
			buildManager,
			() => this.refreshBuilds(),
			() => this.showSettings()
		);
		
		this.statusDisplay = new BlenderStatusDisplay(buildManager);
				this.buildsRenderer = new BlenderBuildsRenderer(
			this.plugin,
			buildManager,
			() => this.refreshBuilds()
		);
		
		// Set up event listeners
		this.setupEventListeners();
	}

	/**
	 * Initialize the layout once
	 */
	initializeLayout(): void {
		if (this.isInitialized) return;
		
		this.layoutManager.initializeLayout();
		this.isInitialized = true;
		
		// Initial render
		this.render();
	}

	/**
	 * Main render method - coordinates all components
	 */
	async render(): Promise<void> {
		if (!this.isInitialized) {
			this.initializeLayout();
		}
		
		// Update toolbar
		this.updateToolbar();
		
		// Update status display
		this.updateStatusDisplay();
		
		// Update builds content
		await this.updateBuildsContent();
	}

	/**
	 * Update toolbar section only
	 */
	private updateToolbar(): void {
		const toolbarContainer = this.layoutManager.getToolbarContainer();
		if (toolbarContainer) {
			this.toolbar.render(toolbarContainer);
		}
	}

	/**
	 * Update status display section only
	 */
	private updateStatusDisplay(): void {
		const statusContainer = this.layoutManager.getStatusContainer();
		if (statusContainer) {
			this.statusDisplay.render(statusContainer);
		}
	}

	/**
	 * Update builds content section only
	 */
	private async updateBuildsContent(): Promise<void> {
		const contentArea = this.layoutManager.getContentArea();
		if (!contentArea) return;
		
		// Get current builds
		const builds = this.buildManager.getCachedBuilds();
		
		// Apply filters
		const filteredBuilds = this.filterBuilds(builds);
		
		// Render builds
		this.buildsRenderer.renderBuilds(contentArea, filteredBuilds);
	}

	/**
	 * Filter builds based on current filter criteria
	 */
	private filterBuilds(builds: BlenderBuildInfo[]): BlenderBuildInfo[] {
		return builds.filter(build => {
			const matchesSearch = !this.currentFilter || 
				build.subversion.toLowerCase().includes(this.currentFilter.toLowerCase()) ||
				build.branch.toLowerCase().includes(this.currentFilter.toLowerCase()) ||
				(build.buildHash && build.buildHash.toLowerCase().includes(this.currentFilter.toLowerCase()));

			const matchesBranch = this.currentBranch === 'all' || build.branch === this.currentBranch;

			return matchesSearch && matchesBranch;
		});
	}

	/**
	 * Refresh builds data
	 */
	async refreshBuilds(): Promise<void> {
		if (this.isRefreshing) return;
		
		this.isRefreshing = true;
		
		try {
			// Update toolbar to show refreshing state
			this.toolbar.setRefreshingState(true);
			
			// Refresh builds data
			await this.buildManager.refreshBuilds();
			
			// Re-render with new data
			await this.updateBuildsContent();
			
		} catch (error) {
			console.error('Failed to refresh builds:', error);
		} finally {
			this.isRefreshing = false;
			this.toolbar.setRefreshingState(false);
		}
	}

	/**
	 * Show settings (placeholder for future implementation)
	 */
	private showSettings(): void {
		// This would open the plugin settings
		// For now, we'll just trigger the existing command
		// @ts-ignore - Using app's internal command system
		this.plugin.app.commands.executeCommandById('app:open-settings');
	}

	/**
	 * Set up event listeners for build manager events
	 */
	private setupEventListeners(): void {
		// Listen for builds updated event
		this.buildManager.on('buildsUpdated', this.onBuildsUpdated.bind(this));
		
		// Listen for scraping status changes
		this.buildManager.on('scrapingStatus', this.onScrapingStatus.bind(this));
		
		// Listen for download events
		this.buildManager.on('downloadStarted', this.onDownloadStarted.bind(this));
		this.buildManager.on('downloadCompleted', this.onDownloadCompleted.bind(this));
	}

	/**
	 * Handle builds updated event
	 */
	private async onBuildsUpdated(builds: BlenderBuildInfo[]): Promise<void> {
		this.cachedBuilds = builds;
		await this.updateBuildsContent();
	}

	/**
	 * Handle scraping status changes
	 */
	private onScrapingStatus(status: any): void {
		// Update status display
		this.updateStatusDisplay();
		
		// Update toolbar state
		this.toolbar.setRefreshingState(status.isActive);
	}

	/**
	 * Handle download started event
	 */
	private onDownloadStarted(build: BlenderBuildInfo, filePath: string): void {
		console.log(`Download started: ${build.subversion}`);
		// Could update UI to show download in progress
	}

	/**
	 * Handle download completed event
	 */
	private onDownloadCompleted(build: BlenderBuildInfo, filePath: string): void {
		console.log(`Download completed: ${build.subversion}`);
		// Could update UI to show completion
	}

	/**
	 * Set search filter
	 */
	setFilter(filter: string): void {
		this.currentFilter = filter;
		this.updateBuildsContent();
	}

	/**
	 * Set branch filter
	 */
	setBranchFilter(branch: string): void {
		this.currentBranch = branch;
		this.updateBuildsContent();
	}

	/**
	 * Get current filter state
	 */
	getFilterState(): { filter: string; branch: string } {
		return {
			filter: this.currentFilter,
			branch: this.currentBranch
		};
	}

	/**
	 * Cleanup method to remove event listeners
	 */
	dispose(): void {
		this.buildManager.removeAllListeners();
	}

	// Expose managers for external access if needed
	getLayoutManager(): BlenderViewLayoutManager { 
		return this.layoutManager; 
	}
	
	getToolbar(): BlenderToolbar { 
		return this.toolbar; 
	}
	
	getStatusDisplay(): BlenderStatusDisplay { 
		return this.statusDisplay; 
	}
	
	getBuildsRenderer(): BlenderBuildsRenderer { 
		return this.buildsRenderer; 
	}
}
