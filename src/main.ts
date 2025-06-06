import { Notice, Plugin, WorkspaceLeaf, addIcon } from 'obsidian';
import { BLENDER_ICON_SVG } from './constants';
import { BlenderPluginSettings, DEFAULT_SETTINGS, FetchBlenderBuildsSettingTab } from './settings';
import { FetchBlenderBuilds } from './buildManager';
import { BlenderBuildsView, BLENDER_BUILDS_VIEW_TYPE } from './views/BlenderBuildsView';

export default class FetchBlenderBuildsPlugin extends Plugin {
	settings: BlenderPluginSettings;
	buildManager: FetchBlenderBuilds;
	async onload() {
		await this.loadSettings();

		// Initialize build manager
		// @ts-ignore - Using Obsidian's internal API
		const vaultPath = this.app.vault.adapter.basePath || this.app.vault.adapter.path || '';
		this.buildManager = new FetchBlenderBuilds(vaultPath, this.settings);

		// Register the view type
		this.registerView(
			BLENDER_BUILDS_VIEW_TYPE,
			(leaf: WorkspaceLeaf) => new BlenderBuildsView(leaf, this, this.buildManager)
		);		// Register custom Blender icon
		addIcon('blender-logo', BLENDER_ICON_SVG);

		// Add ribbon icon with custom Blender logo
		this.addRibbonIcon('blender-logo', 'Open Blender build manager', () => {
			this.openBuildsView();
		});

		// Add command to palette
		this.addCommand({
			id: 'open-blender-builds',
			name: 'Open Blender Builds',
			callback: () => {
				this.openBuildsView();
			}
		});

		// Add command to refresh builds
		this.addCommand({
			id: 'refresh-blender-builds',
			name: 'Refresh Blender Builds',
			callback: async () => {
				new Notice('Refreshing Blender builds...');
				try {
					await this.buildManager.refreshBuilds();
					new Notice('Blender builds refreshed successfully!');
				} catch (error) {
					new Notice(`Failed to refresh builds: ${error.message}`);
				}
			}
		});

		// Add command to debug build detection
		this.addCommand({
			id: 'debug-build-detection',
			name: 'Debug Build Detection',
			callback: async () => {
				try {
					new Notice('Starting build detection debug...', 3000);
					await this.buildManager.forceRefreshExtractedBuilds();
				} catch (error) {
					new Notice(`Failed to debug build detection: ${error.message}`, 8000);
					console.error('[DEBUG] Build detection error:', error);
				}
			}
		});

		// Add command to check paths
		this.addCommand({
			id: 'check-build-paths',
			name: 'Check Build Paths',
			callback: () => {
				const buildsPath = this.buildManager.getBuildsPath();
				const extractsPath = this.buildManager.getExtractsPath();
				
				new Notice(`Builds path: ${buildsPath}`, 8000);
				new Notice(`Extracts path: ${extractsPath}`, 8000);
			}
		});
		
		// Add command to debug cache contents
		this.addCommand({
			id: 'debug-cache-contents',
			name: 'Debug Cache Contents',
			callback: () => {
				this.buildManager.debugShowCacheContents();
			}
		});
		// Add command to open builds directory
		this.addCommand({
			id: 'open-blender-directory',
			name: 'Open Blender Directory',
			callback: () => {
				this.buildManager.openBuildsDirectory();
			}
		});

		// Add command to migrate existing builds
		this.addCommand({
			id: 'migrate-existing-builds',
			name: 'Migrate Existing Builds to Metadata Cache',
			callback: async () => {
				await this.buildManager.scanAndMigrateExistingBuilds();
				new Notice('Migration scan completed - check console for details');
			}
		});

		// Add settings tab
		this.addSettingTab(new FetchBlenderBuildsSettingTab(this.app, this));
	}

	async loadSettings() {
		this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());
	}
	async saveSettings() {
		await this.saveData(this.settings);
		// Update build manager settings
		if (this.buildManager) {
			this.buildManager.updateSettings(this.settings);
		}
		
		// Notify all open Blender builds views to update settings
		const leaves = this.app.workspace.getLeavesOfType(BLENDER_BUILDS_VIEW_TYPE);
		leaves.forEach(leaf => {
			if (leaf.view instanceof BlenderBuildsView) {
				leaf.view.updateSettings();
			}
		});
	}async openBuildsView(): Promise<void> {
		const { workspace } = this.app;

		let leaf: WorkspaceLeaf | null = null;
		const leaves = workspace.getLeavesOfType(BLENDER_BUILDS_VIEW_TYPE);

		if (leaves.length > 0) {
			// A leaf with our view already exists, use that
			leaf = leaves[0];
		} else {
			// No leaf with our view exists, create a new one
			leaf = workspace.getRightLeaf(false);
			if (leaf) {
				await leaf.setViewState({
					type: BLENDER_BUILDS_VIEW_TYPE,
					active: true,
				});
			}
		}

		// "Reveal" the leaf in case it is in a collapsed sidebar
		if (leaf) {
			workspace.revealLeaf(leaf);
		}
	}
}
