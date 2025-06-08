import { BlenderBuildInfo, BuildType } from '../../types';
import { FetchBlenderBuilds } from '../../buildManager';
import { BuildFilter } from '../../buildFilter';
import type BlenderBuildManagerPlugin from '../../main';
import { SearchComponent, ToggleComponent } from 'obsidian';
import { 
	BlenderToolbar, 
	BlenderStatusDisplay, 
	BlenderBuildsRenderer, 
	BlenderViewLayoutManager 
} from '.';
import { debug, info, warn, error, registerLoggerClass } from '../../utils/obsidian-logger';

/**
 * Main renderer component that coordinates all rendering logic for the BlenderBuildsView
 * Inspired by SVNViewRenderer from obsidian-subversion
 */
export class BlenderViewRenderer {
	private plugin: BlenderBuildManagerPlugin;
	private buildManager: FetchBlenderBuilds;	private buildFilter: BuildFilter;
	
	// Component managers
	private layoutManager: BlenderViewLayoutManager;
	private searchComponent: SearchComponent | null = null;
	
	// Component instances
	private toolbar: BlenderToolbar;
	private statusDisplay: BlenderStatusDisplay;
	private buildsRenderer: BlenderBuildsRenderer;	// State handling
	private isInitialized = false;
	private isRefreshing = false;
	private cachedBuilds: BlenderBuildInfo[] = [];
	private currentFilter: string = '';
	private currentBranch: string = 'all';	private currentBuildType: BuildType | 'all' = 'all';
	private showInstalledOnly = false;
	private pinSymlinkedBuild = false;
	private isTypeFilterVisible = false;	constructor(
		plugin: BlenderBuildManagerPlugin,
		buildManager: FetchBlenderBuilds,
		containerEl: HTMLElement
	) {
		registerLoggerClass(this, 'BlenderViewRenderer');
		debug(this, 'BlenderViewRenderer constructor started');
		
		this.plugin = plugin;
		this.buildManager = buildManager;
		this.buildFilter = new BuildFilter(buildManager);
		// Initialize state from settings
		this.showInstalledOnly = plugin.settings.showInstalledOnly;
		this.currentBuildType = plugin.settings.preferredBuildType;
		this.pinSymlinkedBuild = plugin.settings.pinSymlinkedBuild;
		
		debug(this, 'Settings initialized from plugin configuration');
		
		// Initialize layout manager
		this.layoutManager = new BlenderViewLayoutManager(containerEl);
		// Initialize component instances
		debug(this, 'Creating UI component instances');
		this.toolbar = new BlenderToolbar(
			this.plugin,
			buildManager,
			() => this.refreshBuilds(),
			() => this.toggleTypeFilter(),
			() => this.togglePin()
		);
		
		this.statusDisplay = new BlenderStatusDisplay(buildManager);
		this.buildsRenderer = new BlenderBuildsRenderer(
			this.plugin,
			buildManager,
			() => this.refreshUI()
		);
		
		// Set up event listeners
		debug(this, 'Setting up event listeners for build manager');
		this.setupEventListeners();
		
		info(this, 'BlenderViewRenderer constructor completed successfully');
	}
	/**
	 * Initialize the layout once
	 */
	initializeLayout(): void {
		debug(this, `Layout initialization started (already initialized: ${this.isInitialized})`);
		if (this.isInitialized) {
			debug(this, 'Layout already initialized, skipping');
			return;
		}
		
		this.layoutManager.initializeLayout();
		this.isInitialized = true;
		
		// Initial render
		debug(this, 'Calling initial render after layout initialization');
		this.render();
		info(this, 'Layout initialization completed successfully');
	}

	/**
	 * Main render method - coordinates all components
	 */
	async render(): Promise<void> {
		debug(this, `Main render started (initialized: ${this.isInitialized})`);
		
		if (!this.isInitialized) {
			debug(this, 'Renderer not initialized, calling initializeLayout first');
			this.initializeLayout();
		}
		
		// Update toolbar
		debug(this, 'Updating toolbar section');
		this.updateToolbar();
		
		// Update filter section
		debug(this, 'Updating filter section');
		this.updateFilterSection();
		
		// Update status display
		debug(this, 'Updating status display section');
		this.updateStatusDisplay();
		
		// Update builds content
		await this.updateBuildsContent();
	}	/**
	 * Update toolbar section only
	 */
	private updateToolbar(): void {
		const toolbarContainer = this.layoutManager.getToolbarContainer();
		if (toolbarContainer) {
			this.toolbar.render(toolbarContainer);
			this.toolbar.setButtonActive('filter', this.isTypeFilterVisible);
			this.toolbar.setButtonActive('pin', this.pinSymlinkedBuild);
			this.toolbar.updatePinButtonTooltip(this.pinSymlinkedBuild);
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
	 */	private async updateBuildsContent(): Promise<void> {
		const contentArea = this.layoutManager.getContentArea();
		if (!contentArea) return;
				// Get current builds
		const builds = this.buildManager.getCachedBuilds();
		
		// Apply filters
		const filteredBuilds = this.buildFilter.filterBuilds(builds, {
			searchFilter: this.currentFilter,
			branch: this.currentBranch,
			buildType: this.currentBuildType,
			installedOnly: this.showInstalledOnly
		});
		
		// Render builds with highlighting and pin state
		// Pass both filtered builds and all builds so pinned build can be found from unfiltered list
		this.buildsRenderer.renderBuilds(contentArea, filteredBuilds, this.currentFilter, this.pinSymlinkedBuild, builds);
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
		select.addEventListener('change', async (e) => {
			const target = e.target as HTMLSelectElement;
			this.currentBuildType = target.value as BuildType | 'all';
			// Save to settings
			this.plugin.settings.preferredBuildType = this.currentBuildType;
			await this.plugin.saveSettings();
			this.updateBuildsContent();
		});

		// Add "Show installed only" toggle
		const toggleContainer = dropdownContainer.createEl('div', { cls: 'blender-toggle-container' });
		
		const toggleLabel = toggleContainer.createEl('label', { 
			text: 'Show installed only:', 
			cls: 'blender-filter-label' 
		});
				const toggleComponent = new ToggleComponent(toggleContainer);
		toggleComponent.setValue(this.showInstalledOnly);
		toggleComponent.onChange(async (value) => {
			this.showInstalledOnly = value;
			// Save to settings
			this.plugin.settings.showInstalledOnly = value;
			await this.plugin.saveSettings();
			this.updateBuildsContent();
		});

		// Add fuzzy search input using SearchComponent
		const searchContainer = dropdownContainer.createEl('div', { cls: 'blender-search-container' });
		
		// Create SearchComponent instead of plain input
		this.searchComponent = new SearchComponent(searchContainer);
		this.searchComponent.setPlaceholder('Type to filter...');
		this.searchComponent.inputEl.addClass('blender-search-input');
		
		// Set current filter value
		this.searchComponent.setValue(this.currentFilter);
		
		// Add input handler with debouncing
		let searchTimeout: NodeJS.Timeout;
				// Override the onChanged method
		this.searchComponent.onChanged = () => {
			// Clear previous timeout
			if (searchTimeout) {
				clearTimeout(searchTimeout);
			}
			
			// Debounce search for better performance
			searchTimeout = setTimeout(() => {
				this.currentFilter = this.searchComponent?.getValue() || '';
				debug(this, 'Search filter updated (onChanged):', this.currentFilter);
				this.updateBuildsContent();
			}, 300);
		};
				// Also listen to input events directly as a fallback
		this.searchComponent.inputEl.addEventListener('input', () => {
			// Clear previous timeout
			if (searchTimeout) {
				clearTimeout(searchTimeout);
			}
			
			// Debounce search for better performance
			searchTimeout = setTimeout(() => {
				this.currentFilter = this.searchComponent?.getValue() || '';
				debug(this, 'Search filter updated (input event):', this.currentFilter);
				this.updateBuildsContent();
			}, 300);
		});
		
		// Handle immediate search on Enter key
		this.searchComponent.inputEl.addEventListener('keydown', (e) => {
			if (e.key === 'Enter') {
				if (searchTimeout) {
					clearTimeout(searchTimeout);
				}
				this.currentFilter = this.searchComponent?.getValue() || '';
				this.updateBuildsContent();
			}
		});
	}
	

	/**
	 * Toggle pin symlinked build functionality
	 */
	private async togglePin(): Promise<void> {
		this.pinSymlinkedBuild = !this.pinSymlinkedBuild;
		// Save to settings
		this.plugin.settings.pinSymlinkedBuild = this.pinSymlinkedBuild;
		this.plugin.saveSettings();
		this.updateToolbar();
		this.updateBuildsContent();
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
			error(this, 'Failed to refresh builds:', error);
		} finally {
			this.isRefreshing = false;
			this.toolbar.setRefreshingState(false);
		}
	}

	/**
	 * Refresh only the UI display without scraping new data
	 * Useful for when local state changes (install/extract/delete operations)
	 */
	async refreshUI(): Promise<void> {
		await this.updateBuildsContent();
		this.updateStatusDisplay();
	}	/**
	 * Toggle type filter dropdown visibility
	 */
	private toggleTypeFilter(): void {
		this.isTypeFilterVisible = !this.isTypeFilterVisible;
		this.updateToolbar();
		this.updateFilterSection();
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
		
		// Listen for extraction events
		this.buildManager.on('extractionStarted', this.onExtractionStarted.bind(this));
		this.buildManager.on('extractionCompleted', this.onExtractionCompleted.bind(this));
		this.buildManager.on('extractionError', this.onExtractionError.bind(this));
		
		// Listen for build deletion events
		this.buildManager.on('buildDeleted', this.onBuildDeleted.bind(this));
		
		// Listen for settings updates (to refresh view when architecture changes)
		this.buildManager.on('settingsUpdated', this.onSettingsUpdated.bind(this));
	}

	/**
	 * Handle builds updated event
	 */
	private async onBuildsUpdated(builds: BlenderBuildInfo[]): Promise<void> {
		debug(this, '[BlenderViewRenderer] onBuildsUpdated called with', builds.length, 'builds');
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
		debug(this, `Download started: ${build.subversion}`);
		// Could update UI to show download in progress
	}

	/**
	 * Handle download completed event
	 */
	private onDownloadCompleted(build: BlenderBuildInfo, filePath: string): void {
		debug(this, `Download completed: ${build.subversion}`);
		// Could update UI to show completion
	}

	/**
	 * Handle extraction started event
	 */
	private onExtractionStarted(archivePath: string, extractPath: string): void {
		debug(this, `Extraction started: ${archivePath}`);
		// Could update UI to show extraction in progress
	}

	/**
	 * Handle extraction completed event
	 */
	private onExtractionCompleted(archivePath: string, extractPath: string): void {
		debug(this, `Extraction completed: ${archivePath}`);
		// UI refresh is handled by the buildsUpdated event that is emitted after extraction
		// This handler is mainly for logging and potential status updates
	}

	/**
	 * Handle extraction error event
	 */
	private onExtractionError(archivePath: string, error: any): void {
		debug(this, `Extraction error: ${archivePath}`, error);
		// Could update UI to show error state
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
		debug(this, `Build deleted: ${build.subversion}`, deletionResult);
		// Refresh the builds content to update button visibility
		await this.updateBuildsContent();
	}
	
	/**
	 * Update settings and refresh the view
	 */
	updateSettings(): void {
		// Update internal state from plugin settings
		const oldShowInstalledOnly = this.showInstalledOnly;
		const oldBuildType = this.currentBuildType;
		const oldPinSymlinked = this.pinSymlinkedBuild;
		
		this.showInstalledOnly = this.plugin.settings.showInstalledOnly;
		this.currentBuildType = this.plugin.settings.preferredBuildType;
		this.pinSymlinkedBuild = this.plugin.settings.pinSymlinkedBuild;
		
		// Check if any relevant settings changed
		const settingsChanged = (
			oldShowInstalledOnly !== this.showInstalledOnly ||
			oldBuildType !== this.currentBuildType ||
			oldPinSymlinked !== this.pinSymlinkedBuild
		);
		
		if (settingsChanged) {
			// Update toolbar components to reflect new settings
			this.toolbar.updateFromSettings(this.plugin.settings);
			
			// Re-render the view with new settings
			this.render();
		}
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
	}	getBuildsRenderer(): BlenderBuildsRenderer { 
		return this.buildsRenderer; 
	}
	/**
	 * Cleanup method to properly dispose of components
	 */
	cleanup(): void {
		// Reset search component
		if (this.searchComponent) {
			this.searchComponent = null;
		}
	}

}
