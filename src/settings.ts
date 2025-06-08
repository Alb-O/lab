import { App, Notice, PluginSettingTab, Setting } from 'obsidian';
import { MINIMUM_BLENDER_VERSIONS, MinimumBlenderVersionType } from './constants';
import { BuildType } from './types';
import type BlenderBuildManagerPlugin from './main';
import { 
	debug, 
	info, 
	warn, 
	error,
	registerLoggerClass 
} from './utils/obsidian-logger';

export interface BlenderPluginSettings {
	libraryFolder: string;
	autoExtract: boolean;
	cleanUpAfterExtraction: boolean;
	preferredArchitecture: 'auto' | 'x64' | 'arm64';
	minimumBlenderVersion: MinimumBlenderVersionType;
	launchWithConsole: boolean;
	showInstalledOnly: boolean;
	preferredBuildType: BuildType | 'all';
	pinSymlinkedBuild: boolean;
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
	autoExtract: true,
	cleanUpAfterExtraction: true,
	preferredArchitecture: 'auto',
	minimumBlenderVersion: '3.0',
	launchWithConsole: false,
	showInstalledOnly: false,
	preferredBuildType: 'all',
	pinSymlinkedBuild: false,
	blenderEnvironmentVariables: {}
};

export class BlenderBuildManagerSettingsTab extends PluginSettingTab {
	plugin: BlenderBuildManagerPlugin;	constructor(app: App, plugin: BlenderBuildManagerPlugin) {
		super(app, plugin);
		registerLoggerClass(this, 'BlenderBuildManagerSettingsTab');
		debug(this, 'Settings tab constructor started');
		this.plugin = plugin;
		info(this, 'Settings tab constructor completed');
	}	display(): void {
		debug(this, 'Displaying settings tab');
		const { containerEl } = this;

		containerEl.empty();

		debug(this, 'Creating settings controls');
		new Setting(containerEl)
			.setName('Storage directory')
			.setDesc('Directory relative to vault root where Blender builds will be stored. It is highly recommended to prefix the directory with a dot so Obsidian doesn\'t index it.')
			.addText(text => text
				.setPlaceholder('.blender')
				.setValue(this.plugin.settings.libraryFolder)
				.onChange(async (value) => {
					debug(this, `Library folder changed from '${this.plugin.settings.libraryFolder}' to '${value}'`);
					this.plugin.settings.libraryFolder = value;
					await this.plugin.saveSettings();
				}));
		new Setting(containerEl)
			.setName('Auto-extract builds')
			.setDesc('Automatically extract downloaded archives.')
			.addToggle(toggle => toggle
				.setValue(this.plugin.settings.autoExtract)
				.onChange(async (value) => {
					debug(this, `Auto-extract setting changed from ${this.plugin.settings.autoExtract} to ${value}`);
					this.plugin.settings.autoExtract = value;
					await this.plugin.saveSettings();
				}));
		new Setting(containerEl)
			.setName('Clean up after extraction')
			.setDesc('Remove downloaded archives after extraction.')
			.addToggle(toggle => toggle
				.setValue(this.plugin.settings.cleanUpAfterExtraction)
				.onChange(async (value) => {
					debug(this, `Clean up after extraction setting changed from ${this.plugin.settings.cleanUpAfterExtraction} to ${value}`);
					this.plugin.settings.cleanUpAfterExtraction = value;
					await this.plugin.saveSettings();
				}));
		new Setting(containerEl)
			.setName('Launch Blender with console attached')
			.setDesc('Launch the blender.exe executable instead of blender-launcher.exe to keep a console window attached for debugging. This setting only affects Windows.')
			.addToggle(toggle => toggle
				.setValue(this.plugin.settings.launchWithConsole)
				.onChange(async (value) => {
					debug(this, `Launch with console setting changed from ${this.plugin.settings.launchWithConsole} to ${value}`);
					this.plugin.settings.launchWithConsole = value;
					await this.plugin.saveSettings();
				}));
		
		new Setting(containerEl)
			.setName('Minimum Blender version')
			.setDesc('Only show builds with this version or higher.')
			.addDropdown(dropdown => {
				// Add all version options from the constant array
				MINIMUM_BLENDER_VERSIONS.forEach(version => {
					dropdown.addOption(version, version);
				});
				
				return dropdown
					.setValue(this.plugin.settings.minimumBlenderVersion)
					.onChange(async (value: MinimumBlenderVersionType) => {
						this.plugin.settings.minimumBlenderVersion = value;
						await this.plugin.saveSettings();
					});
			});
		new Setting(containerEl)
			.setName('Show installed builds only')
			.setDesc('By default, show only builds that have been downloaded/installed.')
			.addToggle(toggle => toggle
				.setValue(this.plugin.settings.showInstalledOnly)
				.onChange(async (value) => {
					this.plugin.settings.showInstalledOnly = value;
					await this.plugin.saveSettings();
				}));
		new Setting(containerEl)
			.setName('Preferred build type')
			.setDesc('Default build type filter when opening the Blender builds view.')
			.addDropdown(dropdown => {
				dropdown.addOption('all', 'All Types');
				dropdown.addOption(BuildType.Stable, 'Stable');
				dropdown.addOption(BuildType.Daily, 'Daily');
				dropdown.addOption(BuildType.LTS, 'LTS');
				dropdown.addOption(BuildType.Experimental, 'Experimental');
				
				return dropdown
					.setValue(this.plugin.settings.preferredBuildType)
					.onChange(async (value: BuildType | 'all') => {
						this.plugin.settings.preferredBuildType = value;
						await this.plugin.saveSettings();
					});
			});

		new Setting(containerEl)
			.setName('Pin symlinked build')
			.setDesc('Show the currently symlinked build in a separate pinned container at the top.')
			.addToggle(toggle => toggle
				.setValue(this.plugin.settings.pinSymlinkedBuild)
				.onChange(async (value) => {
					this.plugin.settings.pinSymlinkedBuild = value;
					await this.plugin.saveSettings();
				}));

		new Setting(containerEl).setName('Downloads').setHeading();

		new Setting(containerEl)
			.setName('Preferred architecture')
			.setDesc('Architecture preference for downloads.')
			.addDropdown(dropdown => dropdown
				.addOption('auto', 'Auto-detect')
				.addOption('x64', 'x64')
				.addOption('arm64', 'ARM64')
				.setValue(this.plugin.settings.preferredArchitecture)
				.onChange(async (value: 'auto' | 'x64' | 'arm64') => {
					this.plugin.settings.preferredArchitecture = value;
					await this.plugin.saveSettings();
				}));

		new Setting(containerEl).setName('Environment variables').setHeading().setDesc('Set custom path environment variables that will be passed to Blender when launching builds.');

		const envVarDescriptions: Record<string, string> = {
			BLENDER_USER_RESOURCES: 'User-specific resources directory.',
			BLENDER_USER_CONFIG: 'User configuration directory.',
			BLENDER_USER_SCRIPTS: 'User scripts directory.',
			BLENDER_USER_EXTENSIONS: 'User extensions directory.',
			BLENDER_USER_DATAFILES: 'User data files directory.',
			BLENDER_SYSTEM_RESOURCES: 'System resources directory.',
			BLENDER_SYSTEM_SCRIPTS: 'System scripts directory.',
			BLENDER_SYSTEM_EXTENSIONS: 'System extensions directory.',
			BLENDER_SYSTEM_DATAFILES: 'System data files directory.',
			BLENDER_SYSTEM_PYTHON: 'System Python directory.',
			BLENDER_CUSTOM_SPLASH: 'Custom splash screen image.',
			BLENDER_CUSTOM_SPLASH_BANNER: 'Custom splash banner text.',
			OCIO: 'OpenColorIO configuration file.',
			TEMP: 'Temporary files directory.',
			TMPDIR: 'Alternative temporary files directory.'
		};

		Object.entries(envVarDescriptions).forEach(([envVar, description]) => {
			new Setting(containerEl)
				.setName(envVar)
				.setDesc(description)
				.addText(text => text
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
	}
}
