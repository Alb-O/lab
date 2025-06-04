import { BlenderBuildInfo, BuildType } from '../../types';
import { FetchBlenderBuilds } from '../../buildManager';
import { BuildFilter } from '../../buildFilter';
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
	private buildFilter: BuildFilter;
	
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
	private currentBuildType: BuildType | 'all' = 'all';
	private isTypeFilterVisible = false;

	constructor(
		plugin: FetchBlenderBuildsPlugin,
		buildManager: FetchBlenderBuilds,
		containerEl: HTMLElement
	) {
		this.plugin = plugin;
		this.buildManager = buildManager;
		this.buildFilter = new BuildFilter(buildManager);
		
		// Initialize layout manager
		this.layoutManager = new BlenderViewLayoutManager(containerEl);
		// Initialize component instances
		this.toolbar = new BlenderToolbar(
			this.plugin,
			buildManager,
			() => this.refreshBuilds(),
			() => this.showSettings(),
			() => this.toggleTypeFilter()
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
		
		// Update filter section
		this.updateFilterSection();
		
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
			this.toolbar.setButtonActive('filter', this.isTypeFilterVisible);
		}
	}

	/**
	 * Update filter section only
	 */
	private updateFilterSection(): void {
		const filterContainer = this.layoutManager.getFilterContainer();
		if (filterContainer) {
			this.renderTypeFilterDropdown(filterContainer);
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
		const filteredBuilds = this.buildFilter.filterBuilds(builds, {
			searchFilter: this.currentFilter,
			branch: this.currentBranch,
			buildType: this.currentBuildType
		});
		
		// Render builds with highlighting
		this.buildsRenderer.renderBuilds(contentArea, filteredBuilds, this.currentFilter, this.buildFilter.highlightMatches.bind(this.buildFilter));
	}
	

	/**
	 * Render the type filter dropdown
	 */
	private renderTypeFilterDropdown(filterContainer: HTMLElement): void {
		// Clear existing content
		filterContainer.empty();
		
		if (!this.isTypeFilterVisible) {
			return;
		}
		
		// Create dropdown container
		const dropdownContainer = filterContainer.createEl('div', { cls: 'blender-type-filter-dropdown' });
		
		// Create label
		dropdownContainer.createEl('label', { 
			text: 'Filter by build type:', 
			cls: 'blender-filter-label' 
		});
		
		// Create the dropdown
		const select = dropdownContainer.createEl('select', { cls: 'dropdown' });
		
		// Add options
		const options = [
			{ value: 'all', text: 'All Types' },
			{ value: BuildType.Stable, text: 'Stable' },
			{ value: BuildType.Daily, text: 'Daily' },
			{ value: BuildType.LTS, text: 'LTS' },
			{ value: BuildType.Experimental, text: 'Experimental' }
		];
		
		options.forEach(option => {
			const optionEl = select.createEl('option', { 
				value: option.value,
				text: option.text
			});
			if (option.value === this.currentBuildType) {
				optionEl.selected = true;
			}
		});
		
		// Add change handler
		select.addEventListener('change', (e) => {
			const target = e.target as HTMLSelectElement;
			this.currentBuildType = target.value as BuildType | 'all';
			this.updateBuildsContent();
		});

		// Add fuzzy search input
		const searchContainer = dropdownContainer.createEl('div', { cls: 'blender-search-container' });
		
		searchContainer.createEl('label', { 
			text: 'Search builds:', 
			cls: 'blender-filter-label' 
		});
		
		const searchInput = searchContainer.createEl('input', { 
			type: 'text',
			placeholder: 'Search by version, filename, hash, etc...',
			cls: 'blender-search-input'
		});
		
		// Set current filter value
		searchInput.value = this.currentFilter;
		
		// Add input handler with debouncing
		let searchTimeout: NodeJS.Timeout;
		searchInput.addEventListener('input', (e) => {
			const target = e.target as HTMLInputElement;
			
			// Clear previous timeout
			if (searchTimeout) {
				clearTimeout(searchTimeout);
			}
			
			// Debounce search for better performance
			searchTimeout = setTimeout(() => {
				this.currentFilter = target.value;
				this.updateBuildsContent();
			}, 300);
		});
		
		// Handle immediate search on Enter key
		searchInput.addEventListener('keydown', (e) => {
			if (e.key === 'Enter') {
				if (searchTimeout) {
					clearTimeout(searchTimeout);
				}
				this.currentFilter = (e.target as HTMLInputElement).value;
				this.updateBuildsContent();
			}
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
	 * Toggle type filter dropdown visibility
	 */
	private toggleTypeFilter(): void {
		this.isTypeFilterVisible = !this.isTypeFilterVisible;
		this.updateToolbar();
		this.updateFilterSection();
	}
	/**
	 * Set up event listeners for build manager events
	 */	private setupEventListeners(): void {
		// Listen for builds updated event
		this.buildManager.on('buildsUpdated', this.onBuildsUpdated.bind(this));
		
		// Listen for scraping status changes
		this.buildManager.on('scrapingStatus', this.onScrapingStatus.bind(this));
		
		// Listen for download events
		this.buildManager.on('downloadStarted', this.onDownloadStarted.bind(this));
		this.buildManager.on('downloadCompleted', this.onDownloadCompleted.bind(this));
		
		// Listen for build deletion events
		this.buildManager.on('buildDeleted', this.onBuildDeleted.bind(this));
		
		// Listen for settings updates (to refresh view when architecture changes)
		this.buildManager.on('settingsUpdated', this.onSettingsUpdated.bind(this));
	}

	/**
	 * Handle builds updated event
	 */
	private async onBuildsUpdated(builds: BlenderBuildInfo[]): Promise<void> {
		console.log('[BlenderViewRenderer] onBuildsUpdated called with', builds.length, 'builds');
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
	 * Handle settings updated event
	 */
	private async onSettingsUpdated(): Promise<void> {
		// When settings change (like architecture preference), refresh the build list view
		// without requiring a full scrape - just re-filter the existing builds
		await this.updateBuildsContent();
	}

	/**
	 * Handle build deleted event
	 */
	private async onBuildDeleted(build: BlenderBuildInfo, deletionResult: { deletedDownload: boolean; deletedExtract: boolean }): Promise<void> {
		console.log(`Build deleted: ${build.subversion}`, deletionResult);
		// Refresh the builds content to update button visibility
		await this.updateBuildsContent();
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
	 * Set build type filter
	 */
	setBuildTypeFilter(buildType: BuildType | 'all'): void {
		this.currentBuildType = buildType;
		this.updateBuildsContent();
	}

	/**
	 * Get current filter state
	 */
	getFilterState(): { filter: string; branch: string; buildType: BuildType | 'all'; typeFilterVisible: boolean } {
		return {
			filter: this.currentFilter,
			branch: this.currentBranch,
			buildType: this.currentBuildType,
			typeFilterVisible: this.isTypeFilterVisible
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
