import { ItemView, WorkspaceLeaf, App } from 'obsidian';
import { FetchBlenderBuilds } from '../buildManager';
import { BlenderViewRenderer } from '../ui/components';
import type BlenderBuildManagerPlugin from '../main';
import { 
	debug, 
	info, 
	warn, 
	error,
	registerLoggerClass 
} from '../utils/obsidian-logger';

export const BLENDER_BUILDS_VIEW_TYPE = 'blender-builds-view';

export class BlenderBuildsView extends ItemView {
	private plugin: BlenderBuildManagerPlugin;
	private buildManager: FetchBlenderBuilds;
	private viewRenderer: BlenderViewRenderer;
	private isInitialized = false;	constructor(leaf: WorkspaceLeaf, plugin: BlenderBuildManagerPlugin, buildManager: FetchBlenderBuilds) {
		super(leaf);
		registerLoggerClass(this, 'BlenderBuildsView');
		debug(this, 'Creating Blender builds view');
		this.plugin = plugin;
		this.buildManager = buildManager;
		info(this, 'Blender builds view created successfully');
	}

	getViewType(): string {
		return BLENDER_BUILDS_VIEW_TYPE;
	}

	getDisplayText(): string {
		return 'Blender build manager';
	}

	getIcon(): string {
		return 'blender-logo';
	}	async onOpen() {
		debug(this, 'Opening Blender builds view');
		this.initializeView();
		
		// Wait for cached builds to be loaded
		debug(this, 'Waiting for cached builds to load');
		await this.buildManager.waitForCacheLoading();
		
		// Initial render
		debug(this, 'Performing initial view render');
		await this.viewRenderer.render();
		info(this, 'Blender builds view opened successfully');
	}	async onClose() {
		debug(this, 'Closing Blender builds view');
		// Clean up view renderer components
		this.viewRenderer?.cleanup();
		
		// Clean up any event listeners or resources
		this.isInitialized = false;
		info(this, 'Blender builds view closed successfully');
	}
	/**
	 * Initialize the view structure
	 */	private initializeView(): void {
		debug(this, `Initializing view (already initialized: ${this.isInitialized})`);
		if (this.isInitialized) {
			debug(this, 'View already initialized, skipping');
			return;
		}
				// Create the BlenderViewRenderer
		debug(this, 'Creating view renderer component');
		this.viewRenderer = new BlenderViewRenderer(
			this.plugin,
			this.buildManager,
			this.containerEl
		);
		
		// Initialize the layout
		debug(this, 'Initializing view layout');
		this.viewRenderer.initializeLayout();
		this.isInitialized = true;
		info(this, 'View initialization completed');
	}

	/**
	 * Refresh the view
	 */	async refreshView(): Promise<void> {
		debug(this, 'Refreshing view');
		if (this.viewRenderer) {
			await this.viewRenderer.render();
			info(this, 'View refreshed successfully');
		} else {
			warn(this, 'Cannot refresh view - no renderer available');
		}
	}

	/**
	 * Get the view renderer for external access if needed
	 */	getViewRenderer(): BlenderViewRenderer {
		debug(this, 'Getting view renderer');
		return this.viewRenderer;
	}

	/**
	 * Update settings and refresh view
	 */	updateSettings(): void {
		debug(this, 'Updating settings');
		if (this.viewRenderer) {
			this.viewRenderer.updateSettings();
			info(this, 'Settings updated successfully');
		} else {
			warn(this, 'Cannot update settings - no renderer available');
		}
	}
}
