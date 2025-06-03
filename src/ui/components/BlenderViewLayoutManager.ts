import { BlenderToolbar } from '.';

/**
 * Manages the DOM layout and structure for the BlenderBuildsView
 * Inspired by SVNViewLayoutManager
 */
export class BlenderViewLayoutManager {
	private containerEl: HTMLElement;
	private isInitialized = false;
	
	// UI Elements with persistent references
	private toolbarContainer: HTMLElement | null = null;
	private statusContainer: HTMLElement | null = null;
	private contentArea: HTMLElement | null = null;

	constructor(containerEl: HTMLElement) {
		this.containerEl = containerEl;
	}

	/**
	 * Initialize the persistent DOM layout structure
	 */
	initializeLayout(): void {
		if (this.isInitialized) return;
		
		this.containerEl.empty();
		
		// Create persistent container structure (matching SVN plugin)
		this.toolbarContainer = this.containerEl.createEl('div', { cls: 'nav-header' });
		this.statusContainer = this.containerEl.createEl('div', { cls: 'blender-status-display' });
		this.contentArea = this.containerEl.createEl('div', { cls: 'blender-builds-content' });
		
		this.isInitialized = true;
	}

	/**
	 * Update toolbar section only
	 */
	updateToolbar(toolbar: BlenderToolbar): void {
		if (this.toolbarContainer) {
			this.toolbarContainer.empty();
			toolbar.render(this.toolbarContainer);
		}
	}

	/**
	 * Clear the status container
	 */
	clearStatusContainer(): void {
		if (this.statusContainer) {
			this.statusContainer.empty();
		}
	}

	/**
	 * Clear the content area
	 */
	clearContentArea(): void {
		if (this.contentArea) {
			this.contentArea.empty();
		}
	}

	// Getters for UI elements
	getToolbarContainer(): HTMLElement | null { return this.toolbarContainer; }
	getStatusContainer(): HTMLElement | null { return this.statusContainer; }
	getContentArea(): HTMLElement | null { return this.contentArea; }
	
	isLayoutInitialized(): boolean { return this.isInitialized; }

	/**
	 * Reset layout state
	 */
	resetLayout(): void {
		this.isInitialized = false;
		this.toolbarContainer = null;
		this.statusContainer = null;
		this.contentArea = null;
	}
}
