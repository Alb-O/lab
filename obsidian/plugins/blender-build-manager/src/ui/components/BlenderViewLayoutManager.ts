import { BlenderToolbar } from '.';
import { loggerDebug, loggerInfo, loggerWarn, loggerError, registerLoggerClass } from '@/utils/obsidian-logger';

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
	private filterContainer: HTMLElement | null = null;
	private contentArea: HTMLElement | null = null;
	constructor(containerEl: HTMLElement) {
		registerLoggerClass(this, 'BlenderViewLayoutManager');
		this.containerEl = containerEl;
		loggerDebug(this, 'BlenderViewLayoutManager initialized', { 
			containerEl: containerEl.tagName 
		});
	}
	/**
	 * Initialize the persistent DOM layout structure	 */
	initializeLayout(): void {
		if (this.isInitialized) {
			loggerDebug(this, 'Layout already initialized, skipping');
			return;
		}
		
		loggerDebug(this, 'Initializing layout structure');
		this.containerEl.empty();
		this.containerEl.addClass('blender-view-container');
				// Create persistent container structure with proper CSS classes
		this.toolbarContainer = this.containerEl.createEl('div', { cls: 'nav-header' });
		this.statusContainer = this.containerEl.createEl('div', { cls: 'blender-status-display' });
		this.filterContainer = this.containerEl.createEl('div', { cls: 'blender-filter-container' });
		this.contentArea = this.containerEl.createEl('div', { cls: 'blender-content-area' });
		
		this.isInitialized = true;
		loggerInfo(this, 'Layout initialized successfully');
	}

	/**
	 * Update toolbar section only
	 */
	updateToolbar(toolbar: BlenderToolbar): void {
		loggerDebug(this, 'Updating toolbar section');
		if (this.toolbarContainer) {
			this.toolbarContainer.empty();
			toolbar.render(this.toolbarContainer);
		} else {
			loggerWarn(this, 'Toolbar container not initialized');
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
	 * Clear the filter container
	 */
	clearFilterContainer(): void {
		if (this.filterContainer) {
			this.filterContainer.empty();
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
	getFilterContainer(): HTMLElement | null { return this.filterContainer; }
	getContentArea(): HTMLElement | null { return this.contentArea; }
	
	isLayoutInitialized(): boolean { return this.isInitialized; }
	/**
	 * Reset layout state
	 */
	resetLayout(): void {
		this.isInitialized = false;
		this.toolbarContainer = null;
		this.statusContainer = null;
		this.filterContainer = null;
		this.contentArea = null;
	}
}
