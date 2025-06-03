import { 
	App, 
	Notice, 
	Plugin, 
	PluginSettingTab, 
	Setting,
	Platform
} from 'obsidian';

import { BlenderPluginSettings, DEFAULT_SETTINGS } from './types';
import { BlenderBuildManager } from './buildManager';
import { BlenderBuildsModal } from './ui';

export default class BlenderBuildManagerPlugin extends Plugin {
	settings: BlenderPluginSettings;
	buildManager: BlenderBuildManager;

	async onload() {
		await this.loadSettings();		// Initialize build manager
		// @ts-ignore - Using Obsidian's internal API
		const vaultPath = this.app.vault.adapter.basePath || this.app.vault.adapter.path || '';
		this.buildManager = new BlenderBuildManager(vaultPath, this.settings);

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
		this.addSettingTab(new BlenderBuildManagerSettingTab(this.app, this));

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

class BlenderBuildManagerSettingTab extends PluginSettingTab {
	plugin: BlenderBuildManagerPlugin;

	constructor(app: App, plugin: BlenderBuildManagerPlugin) {
		super(app, plugin);
		this.plugin = plugin;
	}

	display(): void {
		const { containerEl } = this;

		containerEl.empty();

		containerEl.createEl('h2', { text: 'Blender Build Manager Settings' });

		// General Settings
		containerEl.createEl('h3', { text: 'General Settings' });
		new Setting(containerEl)
			.setName('Blender Directory')
			.setDesc('Directory relative to vault root where Blender builds will be stored')
			.addText(text => text
				.setPlaceholder('.blender')
				.setValue(this.plugin.settings.libraryFolder)
				.onChange(async (value) => {
					this.plugin.settings.libraryFolder = value;
					await this.plugin.saveSettings();
				}));

		new Setting(containerEl)
			.setName('Auto-extract builds')
			.setDesc('Automatically extract downloaded archives')
			.addToggle(toggle => toggle
				.setValue(this.plugin.settings.autoExtract)
				.onChange(async (value) => {
					this.plugin.settings.autoExtract = value;
					await this.plugin.saveSettings();
				}));

		new Setting(containerEl)
			.setName('Refresh builds on startup')
			.setDesc('Automatically check for new builds when Obsidian starts')
			.addToggle(toggle => toggle
				.setValue(this.plugin.settings.refreshOnStartup)
				.onChange(async (value) => {
					this.plugin.settings.refreshOnStartup = value;
					await this.plugin.saveSettings();
				}));

		// Build Types
		containerEl.createEl('h3', { text: 'Build Types' });
		new Setting(containerEl)
			.setName('Enable Stable Builds')
			.setDesc('Show stable Blender releases')
			.addToggle(toggle => toggle
				.setValue(this.plugin.settings.enableStableBuilds)
				.onChange(async (value) => {
					this.plugin.settings.enableStableBuilds = value;
					await this.plugin.saveSettings();
				}));

		new Setting(containerEl)
			.setName('Enable Daily Builds')
			.setDesc('Show daily development builds')
			.addToggle(toggle => toggle
				.setValue(this.plugin.settings.enableDailyBuilds)
				.onChange(async (value) => {
					this.plugin.settings.enableDailyBuilds = value;
					await this.plugin.saveSettings();
				}));

		new Setting(containerEl)
			.setName('Enable Experimental Builds')
			.setDesc('Show experimental and branch builds')
			.addToggle(toggle => toggle
				.setValue(this.plugin.settings.enableExperimentalBuilds)
				.onChange(async (value) => {
					this.plugin.settings.enableExperimentalBuilds = value;
					await this.plugin.saveSettings();
				}));

		// Download Settings
		containerEl.createEl('h3', { text: 'Download Settings' });

		new Setting(containerEl)
			.setName('Preferred Architecture')
			.setDesc('Architecture preference for downloads')
			.addDropdown(dropdown => dropdown
				.addOption('auto', 'Auto-detect')
				.addOption('x64', 'x64')
				.addOption('arm64', 'ARM64')
				.setValue(this.plugin.settings.preferredArchitecture)
				.onChange(async (value: 'auto' | 'x64' | 'arm64') => {
					this.plugin.settings.preferredArchitecture = value;
					await this.plugin.saveSettings();
				}));

		new Setting(containerEl)
			.setName('Max Concurrent Downloads')
			.setDesc('Maximum number of simultaneous downloads')
			.addSlider(slider => slider
				.setLimits(1, 5, 1)
				.setValue(this.plugin.settings.maxConcurrentDownloads)
				.setDynamicTooltip()
				.onChange(async (value) => {
					this.plugin.settings.maxConcurrentDownloads = value;
					await this.plugin.saveSettings();
				}));

		// Build Management
		containerEl.createEl('h3', { text: 'Build Management' });

		new Setting(containerEl)
			.setName('Keep Old Builds')
			.setDesc('Keep older builds when downloading new ones')
			.addToggle(toggle => toggle
				.setValue(this.plugin.settings.keepOldBuilds)
				.onChange(async (value) => {
					this.plugin.settings.keepOldBuilds = value;
					await this.plugin.saveSettings();
				}));

		new Setting(containerEl)
			.setName('Max Builds to Keep')
			.setDesc('Maximum number of builds to keep (0 = unlimited)')
			.addText(text => text
				.setPlaceholder('10')
				.setValue(this.plugin.settings.maxBuildsToKeep.toString())
				.onChange(async (value) => {
					const num = parseInt(value);
					if (!isNaN(num) && num >= 0) {
						this.plugin.settings.maxBuildsToKeep = num;
						await this.plugin.saveSettings();
					}
				}));

		// Version Filtering
		containerEl.createEl('h3', { text: 'Version Filtering' });
		new Setting(containerEl)
			.setName('Minimum Stable Version')
			.setDesc('Minimum version for stable builds (e.g., "3.0", "4.0")')
			.addText(text => text
				.setPlaceholder('3.0')
				.setValue(this.plugin.settings.minimumBlenderVersion)
				.onChange(async (value) => {
					this.plugin.settings.minimumBlenderVersion = value;
					await this.plugin.saveSettings();
				}));

		// Notifications
		containerEl.createEl('h3', { text: 'Notifications' });

		new Setting(containerEl)
			.setName('Notify on New Builds')
			.setDesc('Show notifications when new builds are available')
			.addToggle(toggle => toggle
				.setValue(this.plugin.settings.showNotifications)
				.onChange(async (value) => {
					this.plugin.settings.showNotifications = value;
					await this.plugin.saveSettings();
				}));

		// Actions
		containerEl.createEl('h3', { text: 'Actions' });

		new Setting(containerEl)
			.setName('Open Blender Directory')
			.setDesc('Open the directory where Blender builds are stored')
			.addButton(button => button
				.setButtonText('Open Directory')
				.setCta()
				.onClick(() => {
					this.plugin.buildManager.openBuildsDirectory();
				}));

		new Setting(containerEl)
			.setName('Refresh Builds')
			.setDesc('Manually refresh the list of available builds')
			.addButton(button => button
				.setButtonText('Refresh Now')
				.onClick(async () => {
					button.setButtonText('Refreshing...');
					button.setDisabled(true);
					try {
						await this.plugin.buildManager.refreshBuilds();
						new Notice('Builds refreshed successfully!');
					} catch (error) {
						new Notice(`Failed to refresh builds: ${error.message}`);
					} finally {
						button.setButtonText('Refresh Now');
						button.setDisabled(false);
					}
				}));

		new Setting(containerEl)
			.setName('Clean Up Old Builds')
			.setDesc('Remove old builds based on the maximum builds setting')
			.addButton(button => button
				.setButtonText('Clean Up')
				.setWarning()
				.onClick(async () => {
					button.setButtonText('Cleaning...');
					button.setDisabled(true);
					try {
						const removed = await this.plugin.buildManager.cleanupOldBuilds();
						new Notice(`Removed ${removed} old builds`);
					} catch (error) {
						new Notice(`Failed to clean up builds: ${error.message}`);
					} finally {
						button.setButtonText('Clean Up');
						button.setDisabled(false);
					}
				}));
	}
}
