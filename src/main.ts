import { Notice, Plugin, WorkspaceLeaf, addIcon } from 'obsidian';
import { BLENDER_ICON_SVG } from './constants';
import { BlenderPluginSettings, DEFAULT_SETTINGS, BlenderBuildManagerSettingsTab } from './settings';
import { FetchBlenderBuilds } from './buildManager';
import { BlenderBuildsView, BLENDER_BUILDS_VIEW_TYPE } from './views/BlenderBuildsView';
import { 
	initLogger, 
	registerLoggerClass, 
	debug, 
	info, 
	warn, 
	error 
} from './utils/obsidian-logger';

export default class BlenderBuildManagerPlugin extends Plugin {
	settings: BlenderPluginSettings;
	buildManager: FetchBlenderBuilds;	async onload() {
		// Initialize the logger system
		initLogger(this);
		registerLoggerClass(this, 'BlenderBuildManagerPlugin');
				debug(this, 'Plugin is starting to load');
		
		await this.loadSettings();
		debug(this, 'Settings have been loaded successfully');
		// Initialize build manager
		debug(this, 'Initializing build manager with vault path');
		// @ts-ignore - Using Obsidian's internal API
		const vaultPath = this.app.vault.adapter.basePath || this.app.vault.adapter.path || '';
		this.buildManager = new FetchBlenderBuilds(vaultPath, this.settings);
		debug(this, 'Build manager created successfully');

		// Register the view type
		debug(this, 'Registering Blender builds view type');
		this.registerView(
			BLENDER_BUILDS_VIEW_TYPE,
			(leaf: WorkspaceLeaf) => new BlenderBuildsView(leaf, this, this.buildManager)
		);
		// Register custom Blender icon
		debug(this, 'Registering custom Blender icon');
		addIcon('blender-logo', BLENDER_ICON_SVG);

		// Add ribbon icon with custom Blender logo
		debug(this, 'Adding ribbon icon for Blender builds');
		this.addRibbonIcon('blender-logo', 'Open Blender build manager', () => {
			debug(this, 'Ribbon icon clicked - opening builds view');
			this.openBuildsView();
		});

		// Add command to palette
		debug(this, 'Registering command palette commands');
		this.addCommand({
			id: 'open-blender-builds',
			name: 'Open Blender Builds',
			callback: () => {
				debug(this, 'Opening Blender builds view from command');
				this.openBuildsView();
			}
		});
		// Add command to refresh builds
		this.addCommand({
			id: 'refresh-blender-builds',
			name: 'Refresh available builds',
			callback: async () => {
				debug(this, 'Starting refresh of Blender builds');
				new Notice('Refreshing Blender builds...');
				try {
					await this.buildManager.refreshBuilds();
					info(this, 'Blender builds refreshed successfully');
					new Notice('Blender builds refreshed successfully!');
				} catch (errorData) {
					error(this, 'Failed to refresh Blender builds', errorData);
					new Notice(`Failed to refresh builds: ${errorData.message}`);
				}
			}
		});
		// Add command to debug build detection
		this.addCommand({
			id: 'debug-build-detection',
			name: 'Debug Build Detection',
			callback: async () => {
				debug(this, 'Starting build detection debug command');
				try {
					new Notice('Starting build detection debug...', 3000);
					await this.buildManager.forceRefreshExtractedBuilds();
					info(this, 'Build detection debug completed successfully');
				} catch (errorData) {
					error(this, 'Build detection debug failed', errorData);
					new Notice(`Failed to debug build detection: ${errorData.message}`, 8000);
					console.error('[DEBUG] Build detection error:', errorData);
				}
			}
		});
		// Add command to check paths
		this.addCommand({
			id: 'check-build-paths',
			name: 'Check Build Paths',
			callback: () => {
				debug(this, 'Checking build paths command started');
				const buildsPath = this.buildManager.getBuildsPath();
				const extractsPath = this.buildManager.getExtractsPath();
				
				debug(this, `Build paths: builds=${buildsPath}, extracts=${extractsPath}`);
				new Notice(`Builds path: ${buildsPath}`, 8000);
				new Notice(`Extracts path: ${extractsPath}`, 8000);
			}
		});
				// Add command to debug cache contents
		this.addCommand({
			id: 'debug-cache-contents',
			name: 'Debug Cache Contents',
			callback: () => {
				debug(this, 'Debugging cache contents');
				this.buildManager.debugShowCacheContents();
			}
		});
		// Add command to open builds directory
		this.addCommand({
			id: 'open-blender-directory',
			name: 'Open Blender Directory',
			callback: () => {
				debug(this, 'Opening Blender directory');
				this.buildManager.openBuildsDirectory();
			}
		});

		// Add command to migrate existing builds
		this.addCommand({
			id: 'migrate-existing-builds',
			name: 'Migrate Existing Builds to Metadata Cache',
			callback: async () => {
				debug(this, 'Starting migration of existing builds to metadata cache');
				await this.buildManager.scanAndMigrateExistingBuilds();
				info(this, 'Build migration completed successfully');
				new Notice('Migration scan completed - check console for details');
			}
		});

		// Add settings tab
		debug(this, 'Adding settings tab to plugin');
		this.addSettingTab(new BlenderBuildManagerSettingsTab(this.app, this));
		
		info(this, 'Plugin loaded successfully');
	}
	async loadSettings() {
		debug(this, 'Loading plugin settings');
		this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());
		debug(this, 'Plugin settings loaded successfully');
	}
	async saveSettings() {
		debug(this, 'Saving plugin settings');
		await this.saveData(this.settings);
		// Update build manager settings
		if (this.buildManager) {
			debug(this, 'Updating build manager with new settings');
			this.buildManager.updateSettings(this.settings);
		}
		
		// Notify all open Blender builds views to update settings
		const leaves = this.app.workspace.getLeavesOfType(BLENDER_BUILDS_VIEW_TYPE);
		debug(this, `Updating ${leaves.length} view(s) with new settings`);
		leaves.forEach(leaf => {
			if (leaf.view instanceof BlenderBuildsView) {
				leaf.view.updateSettings();
			}
		});
		info(this, 'Plugin settings saved successfully');
	}	async openBuildsView(): Promise<void> {
		debug(this, 'Opening Blender builds view');
		const { workspace } = this.app;

		let leaf: WorkspaceLeaf | null = null;
		const leaves = workspace.getLeavesOfType(BLENDER_BUILDS_VIEW_TYPE);
		debug(this, `Found ${leaves.length} existing Blender builds view(s)`);

		if (leaves.length > 0) {
			// A leaf with our view already exists, use that
			leaf = leaves[0];
			debug(this, 'Using existing Blender builds view');
		} else {
			// No leaf with our view exists, create a new one
			debug(this, 'Creating new Blender builds view');
			leaf = workspace.getRightLeaf(false);
			if (leaf) {
				await leaf.setViewState({
					type: BLENDER_BUILDS_VIEW_TYPE,
					active: true,
				});
				debug(this, 'View state set for Blender builds view');
			} else {
				warn(this, 'Failed to get right leaf for Blender builds view');
			}
		}

		// "Reveal" the leaf in case it is in a collapsed sidebar
		if (leaf) {
			debug(this, 'Revealing Blender builds view');
			workspace.revealLeaf(leaf);
			info(this, 'Blender builds view opened successfully');
		} else {
			error(this, 'No leaf available for Blender builds view');
		}
	}
}
