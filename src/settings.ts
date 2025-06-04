import { App, Notice, PluginSettingTab, Setting } from 'obsidian';
import type FetchBlenderBuildsPlugin from './main';

export interface BlenderPluginSettings {
	libraryFolder: string;
	scrapeStableBuilds: boolean;
	scrapeAutomatedBuilds: boolean;
	minimumBlenderVersion: string;
	autoDownloadLatest: boolean;
	showNotifications: boolean;
	maxBuildsToKeep: number;
	autoExtract: boolean;
	cleanupArchives: boolean;
	segregateBuildsByType: boolean;
	// Additional settings for the UI
	enableStableBuilds: boolean;
	enableDailyBuilds: boolean;
	enableExperimentalBuilds: boolean;
	preferredArchitecture: 'auto' | 'x64' | 'arm64';
	maxConcurrentDownloads: number;
	keepOldBuilds: boolean;
	// Blender environment variables
	blenderEnvironmentVariables: {
		BLENDER_USER_RESOURCES?: string;
		BLENDER_USER_CONFIG?: string;
		BLENDER_USER_SCRIPTS?: string;
		BLENDER_USER_EXTENSIONS?: string;
		BLENDER_USER_DATAFILES?: string;
		BLENDER_SYSTEM_RESOURCES?: string;
		BLENDER_SYSTEM_SCRIPTS?: string;
		BLENDER_SYSTEM_EXTENSIONS?: string;
		BLENDER_SYSTEM_DATAFILES?: string;
		BLENDER_SYSTEM_PYTHON?: string;
		BLENDER_CUSTOM_SPLASH?: string;
		BLENDER_CUSTOM_SPLASH_BANNER?: string;
		OCIO?: string;
		TEMP?: string;
		TMPDIR?: string;
	};
}

export const DEFAULT_SETTINGS: BlenderPluginSettings = {
	libraryFolder: '.blender',
	scrapeStableBuilds: true,
	scrapeAutomatedBuilds: true,
	minimumBlenderVersion: '3.0',
	autoDownloadLatest: false,
	showNotifications: true,
	maxBuildsToKeep: 5,
	autoExtract: true,
	cleanupArchives: false,
	segregateBuildsByType: false,
	enableStableBuilds: true,
	enableDailyBuilds: true,
	enableExperimentalBuilds: false,
	preferredArchitecture: 'auto',
	maxConcurrentDownloads: 2,
	keepOldBuilds: true,
	blenderEnvironmentVariables: {}
};

export class FetchBlenderBuildsSettingTab extends PluginSettingTab {
	plugin: FetchBlenderBuildsPlugin;

	constructor(app: App, plugin: FetchBlenderBuildsPlugin) {
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
			.setName('Segregate builds by type')
			.setDesc('Store different types of builds (stable, daily, experimental) in separate subfolders')
			.addToggle(toggle => toggle
				.setValue(this.plugin.settings.segregateBuildsByType)
				.onChange(async (value) => {
					this.plugin.settings.segregateBuildsByType = value;
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
					this.plugin.settings.showNotifications = value;					await this.plugin.saveSettings();
				}));

		// Blender Environment Variables
		containerEl.createEl('h3', { text: 'Blender Environment Variables' });
		containerEl.createEl('p', { 
			text: 'Set custom environment variables that will be passed to Blender when launching builds. Leave empty to use system defaults.',
			cls: 'setting-item-description'
		});

		const envVarDescriptions: Record<string, string> = {
			BLENDER_USER_RESOURCES: 'User-specific resources directory',
			BLENDER_USER_CONFIG: 'User configuration directory',
			BLENDER_USER_SCRIPTS: 'User scripts directory',
			BLENDER_USER_EXTENSIONS: 'User extensions directory',
			BLENDER_USER_DATAFILES: 'User data files directory',
			BLENDER_SYSTEM_RESOURCES: 'System resources directory',
			BLENDER_SYSTEM_SCRIPTS: 'System scripts directory',
			BLENDER_SYSTEM_EXTENSIONS: 'System extensions directory',
			BLENDER_SYSTEM_DATAFILES: 'System data files directory',
			BLENDER_SYSTEM_PYTHON: 'System Python directory',
			BLENDER_CUSTOM_SPLASH: 'Custom splash screen image',
			BLENDER_CUSTOM_SPLASH_BANNER: 'Custom splash banner text',
			OCIO: 'OpenColorIO configuration file',
			TEMP: 'Temporary files directory',
			TMPDIR: 'Alternative temporary files directory'
		};

		Object.entries(envVarDescriptions).forEach(([envVar, description]) => {
			new Setting(containerEl)
				.setName(envVar)
				.setDesc(description)
				.addText(text => text
					.setPlaceholder('Leave empty for system default')
					.setValue(this.plugin.settings.blenderEnvironmentVariables[envVar as keyof typeof this.plugin.settings.blenderEnvironmentVariables] || '')
					.onChange(async (value) => {
						if (value.trim() === '') {
							delete this.plugin.settings.blenderEnvironmentVariables[envVar as keyof typeof this.plugin.settings.blenderEnvironmentVariables];
						} else {
							this.plugin.settings.blenderEnvironmentVariables[envVar as keyof typeof this.plugin.settings.blenderEnvironmentVariables] = value.trim();
						}
						await this.plugin.saveSettings();
					}));
		});

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
						new Notice(`Removed ${removed.removedDownloads} downloads and ${removed.removedExtracts} extracts.`);
					} catch (error) {
						new Notice(`Failed to clean up builds: ${error.message}`);
					} finally {
						button.setButtonText('Clean Up');
						button.setDisabled(false);
					}
				}));
	}
}
