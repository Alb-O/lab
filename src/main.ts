import { Notice, Plugin, WorkspaceLeaf, addIcon } from 'obsidian';
import { BLENDER_ICON_SVG } from './constants';
import { BlenderPluginSettings, DEFAULT_SETTINGS, FetchBlenderBuildsSettingTab } from './settings';
import { FetchBlenderBuilds } from './buildManager';
import { BlenderBuildsView, BLENDER_BUILDS_VIEW_TYPE } from './views/BlenderBuildsView';
import { 
	blenderBuildManagerDebug as debug, 
	blenderBuildManagerInfo as info, 
	blenderBuildManagerWarn as warn, 
	blenderBuildManagerError as error 
} from './debug';

export default class FetchBlenderBuildsPlugin extends Plugin {
	settings: BlenderPluginSettings;
	buildManager: FetchBlenderBuilds;	async onload() {
		debug('plugin', 'onload:start');
		
		await this.loadSettings();
		debug('plugin', 'onload:settings-loaded', this.settings);

		// Initialize build manager
		debug('plugin', 'onload:initializing-build-manager');
		// @ts-ignore - Using Obsidian's internal API
		const vaultPath = this.app.vault.adapter.basePath || this.app.vault.adapter.path || '';
		this.buildManager = new FetchBlenderBuilds(vaultPath, this.settings);
		debug('plugin', 'onload:build-manager-created', { vaultPath });

		// Register the view type
		debug('plugin', 'onload:registering-view');
		this.registerView(
			BLENDER_BUILDS_VIEW_TYPE,
			(leaf: WorkspaceLeaf) => new BlenderBuildsView(leaf, this, this.buildManager)
		);		// Register custom Blender icon
		debug('plugin', 'onload:registering-icon');
		addIcon('blender-logo', BLENDER_ICON_SVG);

		// Add ribbon icon with custom Blender logo
		debug('plugin', 'onload:adding-ribbon-icon');
		this.addRibbonIcon('blender-logo', 'Open Blender build manager', () => {
			debug('ribbon', 'click');
			this.openBuildsView();
		});

		// Add command to palette
		debug('plugin', 'onload:registering-commands');
		this.addCommand({
			id: 'open-blender-builds',
			name: 'Open Blender Builds',
			callback: () => {
				debug('command', 'open-blender-builds');
				this.openBuildsView();
			}
		});
		// Add command to refresh builds
		this.addCommand({
			id: 'refresh-blender-builds',
			name: 'Refresh Blender Builds',
			callback: async () => {
				debug('command', 'refresh-blender-builds:start');
				new Notice('Refreshing Blender builds...');
				try {
					await this.buildManager.refreshBuilds();
					info('command', 'refresh-blender-builds:success');
					new Notice('Blender builds refreshed successfully!');
				} catch (errorData) {
					error('command', 'refresh-blender-builds:failed', errorData);
					new Notice(`Failed to refresh builds: ${errorData.message}`);
				}
			}
		});
		// Add command to debug build detection
		this.addCommand({
			id: 'debug-build-detection',
			name: 'Debug Build Detection',
			callback: async () => {
				debug('command', 'debug-build-detection:start');
				try {
					new Notice('Starting build detection debug...', 3000);
					await this.buildManager.forceRefreshExtractedBuilds();
					info('command', 'debug-build-detection:success');
				} catch (errorData) {
					error('command', 'debug-build-detection:failed', errorData);
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
				debug('command', 'check-build-paths:start');
				const buildsPath = this.buildManager.getBuildsPath();
				const extractsPath = this.buildManager.getExtractsPath();
				
				debug('command', 'check-build-paths:paths', { buildsPath, extractsPath });
				new Notice(`Builds path: ${buildsPath}`, 8000);
				new Notice(`Extracts path: ${extractsPath}`, 8000);
			}
		});
				// Add command to debug cache contents
		this.addCommand({
			id: 'debug-cache-contents',
			name: 'Debug Cache Contents',
			callback: () => {
				debug('command', 'debug-cache-contents');
				this.buildManager.debugShowCacheContents();
			}
		});
		// Add command to open builds directory
		this.addCommand({
			id: 'open-blender-directory',
			name: 'Open Blender Directory',
			callback: () => {
				debug('command', 'open-blender-directory');
				this.buildManager.openBuildsDirectory();
			}
		});

		// Add command to migrate existing builds
		this.addCommand({
			id: 'migrate-existing-builds',
			name: 'Migrate Existing Builds to Metadata Cache',
			callback: async () => {
				debug('command', 'migrate-existing-builds:start');
				await this.buildManager.scanAndMigrateExistingBuilds();
				info('command', 'migrate-existing-builds:completed');
				new Notice('Migration scan completed - check console for details');
			}
		});

		// Add settings tab
		debug('plugin', 'onload:adding-settings-tab');
		this.addSettingTab(new FetchBlenderBuildsSettingTab(this.app, this));
		
		info('plugin', 'onload:complete');
	}
	async loadSettings() {
		debug('settings', 'loadSettings:start');
		this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());
		debug('settings', 'loadSettings:complete', this.settings);
	}
	async saveSettings() {
		debug('settings', 'saveSettings:start', this.settings);
		await this.saveData(this.settings);
		// Update build manager settings
		if (this.buildManager) {
			debug('settings', 'saveSettings:updating-build-manager');
			this.buildManager.updateSettings(this.settings);
		}
		
		// Notify all open Blender builds views to update settings
		const leaves = this.app.workspace.getLeavesOfType(BLENDER_BUILDS_VIEW_TYPE);
		debug('settings', 'saveSettings:updating-views', { leafCount: leaves.length });
		leaves.forEach(leaf => {
			if (leaf.view instanceof BlenderBuildsView) {
				leaf.view.updateSettings();
			}
		});
		info('settings', 'saveSettings:complete');
	}	async openBuildsView(): Promise<void> {
		debug('view', 'openBuildsView:start');
		const { workspace } = this.app;

		let leaf: WorkspaceLeaf | null = null;
		const leaves = workspace.getLeavesOfType(BLENDER_BUILDS_VIEW_TYPE);
		debug('view', 'openBuildsView:existing-leaves', { count: leaves.length });

		if (leaves.length > 0) {
			// A leaf with our view already exists, use that
			leaf = leaves[0];
			debug('view', 'openBuildsView:using-existing-leaf');
		} else {
			// No leaf with our view exists, create a new one
			debug('view', 'openBuildsView:creating-new-leaf');
			leaf = workspace.getRightLeaf(false);
			if (leaf) {
				await leaf.setViewState({
					type: BLENDER_BUILDS_VIEW_TYPE,
					active: true,
				});
				debug('view', 'openBuildsView:view-state-set');
			} else {
				warn('view', 'openBuildsView:failed-to-get-right-leaf');
			}
		}

		// "Reveal" the leaf in case it is in a collapsed sidebar
		if (leaf) {
			debug('view', 'openBuildsView:revealing-leaf');
			workspace.revealLeaf(leaf);
			info('view', 'openBuildsView:complete');
		} else {
			error('view', 'openBuildsView:no-leaf-available');
		}
	}
}
