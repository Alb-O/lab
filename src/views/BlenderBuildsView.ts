import { ItemView, WorkspaceLeaf, App } from 'obsidian';
import { FetchBlenderBuilds } from '../buildManager';
import { BlenderViewRenderer } from '../ui/components';
import type FetchBlenderBuildsPlugin from '../main';

export const BLENDER_BUILDS_VIEW_TYPE = 'blender-builds-view';

export class BlenderBuildsView extends ItemView {
	private plugin: FetchBlenderBuildsPlugin;
	private buildManager: FetchBlenderBuilds;
	private viewRenderer: BlenderViewRenderer;
	private isInitialized = false;

	constructor(leaf: WorkspaceLeaf, plugin: FetchBlenderBuildsPlugin, buildManager: FetchBlenderBuilds) {
		super(leaf);
		this.plugin = plugin;
		this.buildManager = buildManager;
	}

	getViewType(): string {
		return BLENDER_BUILDS_VIEW_TYPE;
	}

	getDisplayText(): string {
		return 'Blender Builds';
	}

	getIcon(): string {
		return 'download';
	}

	async onOpen() {
		this.initializeView();
		
		// Initial render
		await this.viewRenderer.render();
	}

	async onClose() {
		// Clean up any event listeners or resources
		this.isInitialized = false;
	}

	/**
	 * Initialize the view structure
	 */
	private initializeView(): void {
		if (this.isInitialized) return;
				// Create the BlenderViewRenderer
		this.viewRenderer = new BlenderViewRenderer(
			this.plugin,
			this.buildManager,
			this.containerEl
		);
		
		// Initialize the layout
		this.viewRenderer.initializeLayout();
		this.isInitialized = true;
	}

	/**
	 * Refresh the view
	 */
	async refreshView(): Promise<void> {
		if (this.viewRenderer) {
			await this.viewRenderer.render();
		}
	}

	/**
	 * Get the view renderer for external access if needed
	 */
	getViewRenderer(): BlenderViewRenderer {
		return this.viewRenderer;
	}
}
