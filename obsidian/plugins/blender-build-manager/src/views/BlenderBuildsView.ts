import { ItemView, WorkspaceLeaf } from 'obsidian';
import { FetchBlenderBuilds } from '@/build-manager';
import { BlenderViewRenderer } from '@/ui/components';
import type BlenderBuildManagerPlugin from '@/main';
import {
	loggerDebug,
	loggerInfo,
	loggerWarn,
	loggerError,
	registerLoggerClass
} from '@/utils/obsidian-logger';

export const BLENDER_BUILDS_VIEW_TYPE = 'blender-builds-view';

export class BlenderBuildsView extends ItemView {
	private plugin: BlenderBuildManagerPlugin;
	private buildManager: FetchBlenderBuilds;
	private viewRenderer: BlenderViewRenderer;
	private isInitialized = false; constructor(leaf: WorkspaceLeaf, plugin: BlenderBuildManagerPlugin, buildManager: FetchBlenderBuilds) {
		super(leaf);
		registerLoggerClass(this, 'BlenderBuildsView');
		loggerDebug(this, 'Creating Blender builds view');
		this.plugin = plugin;
		this.buildManager = buildManager;
		loggerInfo(this, 'Blender builds view created successfully');
	}

	getViewType(): string {
		return BLENDER_BUILDS_VIEW_TYPE;
	}

	getDisplayText(): string {
		return 'Blender build manager';
	}

	getIcon(): string {
		return 'blender-logo';
	} async onOpen() {
		loggerDebug(this, 'Opening Blender builds view');
		this.initializeView();

		// Wait for cached builds to be loaded
		loggerDebug(this, 'Waiting for cached builds to load');
		await this.buildManager.waitForCacheLoading();

		// Initial render
		loggerDebug(this, 'Performing initial view render');
		await this.viewRenderer.render();
		loggerInfo(this, 'Blender builds view opened successfully');
	} async onClose() {
		loggerDebug(this, 'Closing Blender builds view');
		// Clean up view renderer components
		this.viewRenderer?.cleanup();

		// Clean up any event listeners or resources
		this.isInitialized = false;
		loggerInfo(this, 'Blender builds view closed successfully');
	}
	
	/**
	 * Initialize the view structure
	 */
	private initializeView(): void {
		loggerDebug(this, `Initializing view (already initialized: ${this.isInitialized})`);
		if (this.isInitialized) {
			loggerDebug(this, 'View already initialized, skipping');
			return;
		}
		// Create the BlenderViewRenderer
		loggerDebug(this, 'Creating view renderer component');
		this.viewRenderer = new BlenderViewRenderer(
			this.plugin,
			this.buildManager,
			this.containerEl
		);

		// Initialize the layout
		loggerDebug(this, 'Initializing view layout');
		this.viewRenderer.initializeLayout();
		this.isInitialized = true;
		loggerInfo(this, 'View initialization completed');
	}

	/**
	 * Refresh the view
	 */
	async refreshView(): Promise<void> {
		loggerDebug(this, 'Refreshing view');
		if (this.viewRenderer) {
			await this.viewRenderer.render();
			loggerInfo(this, 'View refreshed successfully');
		} else {
			loggerWarn(this, 'Cannot refresh view - no renderer available');
		}
	}

	/**
	 * Get the view renderer for external access if needed
	 */
	getViewRenderer(): BlenderViewRenderer {
		loggerDebug(this, 'Getting view renderer');
		return this.viewRenderer;
	}

	/**
	 * Update settings and refresh view
	 */	updateSettings(): void {
		loggerDebug(this, 'Updating settings');
		if (this.viewRenderer) {
			this.viewRenderer.updateSettings();
			loggerInfo(this, 'Settings updated successfully');
		} else {
			loggerWarn(this, 'Cannot update settings - no renderer available');
		}
	}
}