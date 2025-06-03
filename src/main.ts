import { Notice, Plugin, WorkspaceLeaf } from 'obsidian';
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
		);

		// Add ribbon icon
		this.addRibbonIcon('download', 'Blender Build Manager', (evt: MouseEvent) => {
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

		// Add command to open builds directory
		this.addCommand({
			id: 'open-blender-directory',
			name: 'Open Blender Directory',
			callback: () => {
				this.buildManager.openBuildsDirectory();
			}
		});

		// Add settings tab
		this.addSettingTab(new FetchBlenderBuildsSettingTab(this.app, this));

		console.log('Blender Build Manager plugin loaded');
	}

	onunload() {
		console.log('Blender Build Manager plugin unloaded');
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
	}	async openBuildsView(): Promise<void> {
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
