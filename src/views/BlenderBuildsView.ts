import { ItemView, WorkspaceLeaf, App } from 'obsidian';
import { FetchBlenderBuilds } from '../buildManager';
import { BlenderViewRenderer } from '../ui/components';
import type FetchBlenderBuildsPlugin from '../main';
import { 
	blenderBuildManagerDebug as debug, 
	blenderBuildManagerInfo as info, 
	blenderBuildManagerWarn as warn, 
	blenderBuildManagerError as error 
} from '../debug';

export const BLENDER_BUILDS_VIEW_TYPE = 'blender-builds-view';

export class BlenderBuildsView extends ItemView {
	private plugin: FetchBlenderBuildsPlugin;
	private buildManager: FetchBlenderBuilds;
	private viewRenderer: BlenderViewRenderer;
	private isInitialized = false;
	constructor(leaf: WorkspaceLeaf, plugin: FetchBlenderBuildsPlugin, buildManager: FetchBlenderBuilds) {
		super(leaf);
		debug('view', 'constructor:start');
		this.plugin = plugin;
		this.buildManager = buildManager;
		info('view', 'constructor:complete');
	}

	getViewType(): string {
		return BLENDER_BUILDS_VIEW_TYPE;
	}

	getDisplayText(): string {
		return 'Blender build manager';
	}

	getIcon(): string {
		return 'blender-logo';
	}
	async onOpen() {
		debug('view', 'onOpen:start');
		this.initializeView();
		
		// Wait for cached builds to be loaded
		debug('view', 'onOpen:waiting-for-cache');
		await this.buildManager.waitForCacheLoading();
		
		// Initial render
		debug('view', 'onOpen:initial-render');
		await this.viewRenderer.render();
		info('view', 'onOpen:complete');
	}
	async onClose() {
		debug('view', 'onClose:start');
		// Clean up view renderer components
		this.viewRenderer?.cleanup();
		
		// Clean up any event listeners or resources
		this.isInitialized = false;
		info('view', 'onClose:complete');
	}
	/**
	 * Initialize the view structure
	 */
	private initializeView(): void {
		debug('view', 'initializeView:start', { isInitialized: this.isInitialized });
		if (this.isInitialized) {
			debug('view', 'initializeView:already-initialized');
			return;
		}
				// Create the BlenderViewRenderer
		debug('view', 'initializeView:creating-renderer');
		this.viewRenderer = new BlenderViewRenderer(
			this.plugin,
			this.buildManager,
			this.containerEl
		);
		
		// Initialize the layout
		debug('view', 'initializeView:initializing-layout');
		this.viewRenderer.initializeLayout();
		this.isInitialized = true;
		info('view', 'initializeView:complete');
	}

	/**
	 * Refresh the view
	 */
	async refreshView(): Promise<void> {
		debug('view', 'refreshView:start');
		if (this.viewRenderer) {
			await this.viewRenderer.render();
			info('view', 'refreshView:complete');
		} else {
			warn('view', 'refreshView:no-renderer');
		}
	}

	/**
	 * Get the view renderer for external access if needed
	 */
	getViewRenderer(): BlenderViewRenderer {
		debug('view', 'getViewRenderer');
		return this.viewRenderer;
	}

	/**
	 * Update settings and refresh view
	 */
	updateSettings(): void {
		debug('view', 'updateSettings:start');
		if (this.viewRenderer) {
			this.viewRenderer.updateSettings();
			info('view', 'updateSettings:complete');
		} else {
			warn('view', 'updateSettings:no-renderer');
		}
	}
}
