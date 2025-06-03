import { BlenderBuildInfo, BuildType } from '../../types';
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
	private currentBuildType: BuildType | 'all' = 'all';
	private isTypeFilterVisible = false;

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
	}	/**
	 * Update builds content section only
	 */
	private async updateBuildsContent(): Promise<void> {
		const contentArea = this.layoutManager.getContentArea();
		if (!contentArea) return;
		
		// Get current builds
		const builds = this.buildManager.getCachedBuilds();
		
		// Apply filters
		const filteredBuilds = this.filterBuilds(builds);
		
		// Render builds with highlighting
		this.buildsRenderer.renderBuilds(contentArea, filteredBuilds, this.currentFilter, this.highlightMatches.bind(this));
	}	/**
	 * More strict fuzzy search implementation
	 */
	private fuzzyMatch(needle: string, haystack: string): boolean {
		if (!needle) return true;
		
		const needleLower = needle.toLowerCase();
		const haystackLower = haystack.toLowerCase();
		
		// Simple substring match first (faster for exact matches)
		if (haystackLower.includes(needleLower)) {
			return true;
		}
		
		// For longer search terms, be more strict about fuzzy matching
		if (needleLower.length > 6) {
			return false; // No fuzzy matching for long terms
		}
		
		// Fuzzy match with gap constraints
		let needleIndex = 0;
		let lastMatchIndex = -1;
		const maxGap = Math.max(2, Math.floor(needleLower.length / 2)); // Allow max gap of 2 or half the needle length
		
		for (let i = 0; i < haystackLower.length && needleIndex < needleLower.length; i++) {
			if (haystackLower[i] === needleLower[needleIndex]) {
				// Check if gap is too large
				if (lastMatchIndex >= 0 && (i - lastMatchIndex) > maxGap) {
					return false;
				}
				lastMatchIndex = i;
				needleIndex++;
			}
		}
		
		// Only consider it a match if we matched all characters
		// and the total span isn't too spread out
		if (needleIndex === needleLower.length && lastMatchIndex >= 0) {
			const span = lastMatchIndex - (haystackLower.indexOf(needleLower[0]));
			const maxSpan = needleLower.length * 3; // Allow max span of 3x the needle length
			return span <= maxSpan;
		}
		
		return false;
	}

	/**
	 * Highlight matching characters in text for fuzzy search
	 */
	highlightMatches(needle: string, haystack: string): string {
		if (!needle || !this.fuzzyMatch(needle, haystack)) {
			return haystack;
		}
		
		const needleLower = needle.toLowerCase();
		const haystackLower = haystack.toLowerCase();
				// For exact substring matches, highlight the entire match
		if (haystackLower.includes(needleLower)) {
			const startIndex = haystackLower.indexOf(needleLower);
			const endIndex = startIndex + needleLower.length;
			return haystack.substring(0, startIndex) + 
				   '<strong style="color: var(--text-normal);">' + haystack.substring(startIndex, endIndex) + '</strong>' + 
				   haystack.substring(endIndex);
		}
		
		// For fuzzy matches, highlight individual matching characters
		let result = '';
		let needleIndex = 0;
		
		for (let i = 0; i < haystack.length; i++) {
			const char = haystack[i];
			if (needleIndex < needleLower.length && 
				char.toLowerCase() === needleLower[needleIndex]) {
				result += '<strong style="color: var(--text-normal);">' + char + '</strong>';
				needleIndex++;
			} else {
				result += char;
			}
		}
		
		return result;
	}

	/**
	 * Extract searchable text from build info
	 */
	private getSearchableText(build: BlenderBuildInfo): string {
		const filename = build.link.split('/').pop() || '';
		const parts = [
			build.subversion,
			build.branch,
			build.buildHash || '',
			filename,
			build.commitTime.toLocaleDateString(),
			build.commitTime.toLocaleString()
		];
		
		return parts.join(' ').toLowerCase();
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
	}	/**
	 * Filter builds based on current filter criteria
	 */	private filterBuilds(builds: BlenderBuildInfo[]): BlenderBuildInfo[] {
		let filteredBuilds = builds.filter(build => {
			const matchesBranch = this.currentBranch === 'all' || build.branch === this.currentBranch;
			const matchesBuildType = this.currentBuildType === 'all' || this.getBuildType(build) === this.currentBuildType;
			
			// If no search filter, just apply branch and type filters
			if (!this.currentFilter) {
				return matchesBranch && matchesBuildType;
			}
			
			// Apply search filter with improved matching
			const matchesSearch = this.improvedFuzzyMatch(build, this.currentFilter);
			
			return matchesSearch && matchesBranch && matchesBuildType;
		});
		
		// Sort results by relevance when there's a search filter
		if (this.currentFilter) {
			filteredBuilds = this.sortByRelevance(filteredBuilds, this.currentFilter);
		}
		
		return filteredBuilds;
	}

	/**
	 * Determine build type from build info
	 */
	private getBuildType(build: BlenderBuildInfo): BuildType {
		const version = build.subversion.toLowerCase();
		const branch = build.branch.toLowerCase();
		
		// Check for stable releases (e.g., "4.2.0", "3.6.5")
		if (/^\d+\.\d+\.\d+$/.test(version) && !version.includes('alpha') && !version.includes('beta') && !version.includes('rc')) {
			return BuildType.Stable;
		}
		
		// Check for LTS releases
		if (version.includes('lts') || branch.includes('lts')) {
			return BuildType.LTS;
		}
		
		// Check for experimental builds
		if (branch.includes('experimental') || version.includes('experimental')) {
			return BuildType.Experimental;
		}
		
		// Default to daily builds for everything else
		return BuildType.Daily;
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

	/**
	 * Improved fuzzy matching that prioritizes exact matches and relevance
	 */
	private improvedFuzzyMatch(build: BlenderBuildInfo, searchTerm: string): boolean {
		const searchLower = searchTerm.toLowerCase();
		
		// Get individual searchable fields
		const filename = build.link.split('/').pop() || '';
		const fields = {
			version: build.subversion,
			branch: build.branch,
			hash: build.buildHash || '',
			filename: filename,
			date: build.commitTime.toLocaleDateString(),
			datetime: build.commitTime.toLocaleString()
		};
		
		// Check for exact substring matches in any field (highest priority)
		for (const [key, value] of Object.entries(fields)) {
			if (value.toLowerCase().includes(searchLower)) {
				return true;
			}
		}
				// Check for fuzzy matches, but be much more restrictive
		// Only allow fuzzy matching for very short search terms (3 chars or less)
		// and only if there's a reasonable match in a single field
		if (searchTerm.length <= 3) {
			for (const [key, value] of Object.entries(fields)) {
				if (this.fuzzyMatch(searchTerm, value)) {
					return true;
				}
			}
		}
		
		return false;
	}
	
	/**
	 * Sort filtered builds by search relevance
	 */
	private sortByRelevance(builds: BlenderBuildInfo[], searchTerm: string): BlenderBuildInfo[] {
		const searchLower = searchTerm.toLowerCase();
		
		return builds.sort((a, b) => {
			const scoreA = this.getRelevanceScore(a, searchLower);
			const scoreB = this.getRelevanceScore(b, searchLower);
			
			// Higher scores come first
			return scoreB - scoreA;
		});
	}
	
	/**
	 * Calculate relevance score for a build based on search term
	 */
	private getRelevanceScore(build: BlenderBuildInfo, searchLower: string): number {
		let score = 0;
		const filename = build.link.split('/').pop() || '';
		
		const fields = [
			{ value: build.buildHash || '', weight: 10 },      // Hash matches are very specific
			{ value: build.subversion, weight: 8 },            // Version matches are important
			{ value: filename, weight: 6 },                    // Filename matches are relevant
			{ value: build.branch, weight: 4 },                // Branch matches are less specific
			{ value: build.commitTime.toLocaleDateString(), weight: 2 }, // Date matches are least specific
			{ value: build.commitTime.toLocaleString(), weight: 1 }
		];
		
		for (const field of fields) {
			const fieldLower = field.value.toLowerCase();
			
			// Exact match gets full weight
			if (fieldLower === searchLower) {
				score += field.weight * 10;
			}
			// Starts with search term gets high weight
			else if (fieldLower.startsWith(searchLower)) {
				score += field.weight * 5;
			}
			// Contains search term gets medium weight
			else if (fieldLower.includes(searchLower)) {
				score += field.weight * 3;
			}
			// Fuzzy match gets low weight
			else if (this.fuzzyMatch(searchLower, fieldLower)) {
				score += field.weight * 1;
			}
		}
		
		return score;
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
