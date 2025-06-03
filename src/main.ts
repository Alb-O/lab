import { Notice, Plugin } from 'obsidian';
import { BlenderPluginSettings, DEFAULT_SETTINGS, FetchBlenderBuildsSettingTab } from './settings';
import { FetchBlenderBuilds } from './buildManager';
import { BlenderBuildsModal } from './ui';

export default class FetchBlenderBuildsPlugin extends Plugin {
	settings: BlenderPluginSettings;
	buildManager: FetchBlenderBuilds;

	async onload() {
		await this.loadSettings();

		// Initialize build manager
		// @ts-ignore - Using Obsidian's internal API
		const vaultPath = this.app.vault.adapter.basePath || this.app.vault.adapter.path || '';
		this.buildManager = new FetchBlenderBuilds(vaultPath, this.settings);

		// Add ribbon icon
		this.addRibbonIcon('download', 'Blender Build Manager', (evt: MouseEvent) => {
			this.openBuildsModal();
		});

		// Add command to palette
		this.addCommand({
			id: 'open-blender-builds',
			name: 'Open Blender Builds',
			callback: () => {
				this.openBuildsModal();
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

		// Initialize builds on startup if enabled
		if (this.settings.refreshOnStartup) {
			this.buildManager.refreshBuilds().catch(error => {
				console.error('Failed to refresh builds on startup:', error);
			});
		}

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
	}

	private openBuildsModal() {
		new BlenderBuildsModal(this.app, this.buildManager).open();
	}
}
