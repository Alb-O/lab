import { Notice, Plugin, WorkspaceLeaf, addIcon } from 'obsidian';
import { initLogger, registerLoggerClass, loggerDebug, loggerInfo, loggerWarn, loggerError, initializeDebugSystem } from '@utils/obsidian-logger';
import { BLENDER_ICON_SVG } from '@constants';
import { BlenderPluginSettings, DEFAULT_SETTINGS, BlenderBuildManagerSettingsTab } from '@settings';
import { BuildManager } from '@build-manager';
import { BlenderBuildsView, BLENDER_BUILDS_VIEW_TYPE } from '@views/BlenderBuildsView';

export default class BlenderBuildManagerPlugin extends Plugin {
	settings: BlenderPluginSettings;
	buildManager: BuildManager;	async onload() {
		// Initialize the logger system
		initLogger(this);
		registerLoggerClass(this, 'BlenderBuildManagerPlugin');

		loggerInfo(this, 'Blender Build Manager Plugin starting initialization', { version: this.manifest.version });

		try {
			loggerDebug(this, 'Loading plugin settings from storage');
			await this.loadSettings();
			
			// Initialize build manager
			loggerDebug(this, 'Creating build manager instance');
			// @ts-ignore - Using Obsidian's internal API
			const vaultPath = this.app.vault.adapter.basePath || this.app.vault.adapter.path || '';
			this.buildManager = new BuildManager(vaultPath, this.settings);
			
			loggerDebug(this, 'Registering build manager with logger for enhanced debugging');
			registerLoggerClass(this.buildManager, 'BuildManager');
			
			loggerDebug(this, 'Initializing plugin components');
			this.initializeComponents();

			loggerInfo(this, 'Blender Build Manager Plugin successfully loaded and ready', { 
				settingsLoaded: !!this.settings,
				buildManagerInitialized: !!this.buildManager
			});
		} catch (initError) {
			loggerError(this, 'Failed to initialize Blender Build Manager Plugin', { 
				error: initError instanceof Error ? initError.message : String(initError),
				stack: initError instanceof Error ? initError.stack : undefined
			});
			throw initError;
		}

		this.app.workspace.onLayoutReady(() => {
      		initializeDebugSystem();
   	 	});
	}
	private initializeComponents() {
		loggerDebug(this, 'Registering Blender builds view type');
		this.registerView(
			BLENDER_BUILDS_VIEW_TYPE,
			(leaf: WorkspaceLeaf) => new BlenderBuildsView(leaf, this, this.buildManager)
		);
		
		loggerDebug(this, 'Registering custom Blender icon');
		addIcon('blender-logo', BLENDER_ICON_SVG);

		loggerDebug(this, 'Adding ribbon icon for Blender builds');
		this.addRibbonIcon('blender-logo', 'Open Blender build manager', (evt: MouseEvent) => {
			loggerDebug(this, 'User clicked ribbon icon', { mouseEvent: evt.type });
			this.handleRibbonClick();
		});

		loggerDebug(this, 'Registering command palette commands');
		this.registerCommands();
		
		loggerDebug(this, 'Adding settings tab to plugin');
		this.addSettingTab(new BlenderBuildManagerSettingsTab(this.app, this));
	}

	private handleRibbonClick() {
		loggerDebug(this, 'Processing ribbon icon click');
		try {
			loggerInfo(this, 'Ribbon click processed successfully - opening builds view');
			this.openBuildsView();
		} catch (ribbonError) {
			loggerError(this, 'Error handling ribbon click', { 
				error: ribbonError instanceof Error ? ribbonError.message : String(ribbonError) 
			});
		}
	}

	private registerCommands() {
		this.addCommand({
			id: 'open-blender-builds',
			name: 'Open Blender Builds',
			callback: () => {
				loggerInfo(this, 'User executed open blender builds command');
				this.executeOpenBuildsCommand();
			}
		});

		this.addCommand({
			id: 'refresh-blender-builds',
			name: 'Refresh available builds',
			callback: async () => {
				loggerInfo(this, 'User executed refresh builds command');
				this.executeRefreshBuildsCommand();
			}
		});

		this.addCommand({
			id: 'debug-build-detection',
			name: 'Debug Build Detection',
			callback: async () => {
				loggerInfo(this, 'User executed debug build detection command');
				this.executeDebugBuildDetectionCommand();
			}
		});

		this.addCommand({
			id: 'check-build-paths',
			name: 'Check Build Paths',
			callback: () => {
				loggerInfo(this, 'User executed check build paths command');
				this.executeCheckBuildPathsCommand();
			}
		});
	
		this.addCommand({
			id: 'debug-cache-contents',
			name: 'Debug Cache Contents',
			callback: () => {
				loggerInfo(this, 'User executed debug cache contents command');
				this.executeDebugCacheContentsCommand();
			}
		});

		this.addCommand({
			id: 'open-blender-directory',
			name: 'Open Blender Directory',
			callback: () => {
				loggerInfo(this, 'User executed open blender directory command');
				this.executeOpenBlenderDirectoryCommand();
			}
		});

		this.addCommand({
			id: 'migrate-existing-builds',
			name: 'Migrate Existing Builds to Metadata Cache',
			callback: async () => {
				loggerInfo(this, 'User executed migrate existing builds command');
				this.executeMigrateExistingBuildsCommand();
			}
		});
	}

	private executeOpenBuildsCommand() {
		loggerDebug(this, 'Executing open Blender builds command');
		try {
			this.openBuildsView();
			loggerInfo(this, 'Open builds command executed successfully');
		} catch (commandError) {
			loggerError(this, 'Failed to execute open builds command', { 
				error: commandError instanceof Error ? commandError.message : String(commandError)
			});
		}
	}

	private async executeRefreshBuildsCommand() {
		loggerDebug(this, 'Executing refresh Blender builds command');
		new Notice('Refreshing Blender builds...');
		try {
			await this.buildManager.refreshBuilds();
			loggerInfo(this, 'Refresh builds command executed successfully');
			new Notice('Blender builds refreshed successfully!');
		} catch (commandError) {
			loggerError(this, 'Failed to execute refresh builds command', { 
				error: commandError instanceof Error ? commandError.message : String(commandError)
			});
			new Notice(`Failed to refresh builds: ${commandError.message}`);
		}
	}

	private async executeDebugBuildDetectionCommand() {
		loggerDebug(this, 'Executing debug build detection command');
		try {
			new Notice('Starting build detection debug...', 3000);
			await this.buildManager.forceRefreshExtractedBuilds();
			loggerInfo(this, 'Debug build detection command executed successfully');
		} catch (commandError) {
			loggerError(this, 'Failed to execute debug build detection command', { 
				error: commandError instanceof Error ? commandError.message : String(commandError)
			});
			new Notice(`Failed to debug build detection: ${commandError.message}`, 8000);
			// Keep one console.error for severe debugging issues
			loggerError(this, '[DEBUG] Build detection error:', commandError);
		}
	}

	private executeCheckBuildPathsCommand() {
		loggerDebug(this, 'Executing check build paths command');
		try {
			const buildsPath = this.buildManager.getBuildsPath();
			const extractsPath = this.buildManager.getExtractsPath();
			
			loggerInfo(this, 'Check build paths command executed successfully', { 
				buildsPath, 
				extractsPath 
			});
			new Notice(`Builds path: ${buildsPath}`, 8000);
			new Notice(`Extracts path: ${extractsPath}`, 8000);
		} catch (commandError) {
			loggerError(this, 'Failed to execute check build paths command', { 
				error: commandError instanceof Error ? commandError.message : String(commandError)
			});
		}
	}

	private executeDebugCacheContentsCommand() {
		loggerDebug(this, 'Executing debug cache contents command');
		try {
			this.buildManager.debugShowCacheContents();
			loggerInfo(this, 'Debug cache contents command executed successfully');
		} catch (commandError) {
			loggerError(this, 'Failed to execute debug cache contents command', { 
				error: commandError instanceof Error ? commandError.message : String(commandError)
			});
		}
	}

	private executeOpenBlenderDirectoryCommand() {
		loggerDebug(this, 'Executing open Blender directory command');
		try {
			this.buildManager.openBuildsDirectory();
			loggerInfo(this, 'Open Blender directory command executed successfully');
		} catch (commandError) {
			loggerError(this, 'Failed to execute open Blender directory command', { 
				error: commandError instanceof Error ? commandError.message : String(commandError)
			});
		}
	}

	private async executeMigrateExistingBuildsCommand() {
		loggerDebug(this, 'Executing migrate existing builds command');
		try {
			await this.buildManager.scanAndMigrateExistingBuilds();
			loggerInfo(this, 'Migrate existing builds command executed successfully');
			new Notice('Migration scan completed - check console for details');
		} catch (commandError) {
			loggerError(this, 'Failed to execute migrate existing builds command', { 
				error: commandError instanceof Error ? commandError.message : String(commandError)
			});
		}	}

	async loadSettings() {
		loggerDebug(this, 'Loading plugin settings from storage');
		this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());
		loggerInfo(this, 'Plugin settings loaded successfully', { 
			settingsKeys: Object.keys(this.settings),
			libraryFolder: this.settings.libraryFolder 
		});
	}

	async saveSettings() {
		loggerDebug(this, 'Attempting to save plugin settings to storage');
		try {
			await this.saveData(this.settings);
			loggerInfo(this, 'Plugin settings successfully saved to storage', { 
				settingsKeys: Object.keys(this.settings),
				libraryFolder: this.settings.libraryFolder 
			});
			
			// Update build manager settings
			if (this.buildManager) {
				loggerDebug(this, 'Updating build manager with new settings');
				this.buildManager.updateSettings(this.settings);
			}
			
			// Notify all open Blender builds views to update settings
			const leaves = this.app.workspace.getLeavesOfType(BLENDER_BUILDS_VIEW_TYPE);
			loggerDebug(this, `Updating ${leaves.length} view(s) with new settings`);
			leaves.forEach(leaf => {
				if (leaf.view instanceof BlenderBuildsView) {
					leaf.view.updateSettings();
				}
			});
		} catch (saveError) {
			loggerError(this, 'Failed to save plugin settings to storage', { 
				error: saveError instanceof Error ? saveError.message : String(saveError),
				settings: this.settings 
			});
			throw saveError;
		}
	}	async openBuildsView(): Promise<void> {
		loggerDebug(this, 'Processing request to open Blender builds view');
		try {
			const { workspace } = this.app;

			let leaf: WorkspaceLeaf | null = null;
			const leaves = workspace.getLeavesOfType(BLENDER_BUILDS_VIEW_TYPE);
			loggerDebug(this, `Found ${leaves.length} existing Blender builds view(s)`);

			if (leaves.length > 0) {
				// A leaf with our view already exists, use that
				leaf = leaves[0];
				loggerDebug(this, 'Using existing Blender builds view');
			} else {
				// No leaf with our view exists, create a new one
				loggerDebug(this, 'Creating new Blender builds view in right sidebar');
				leaf = workspace.getRightLeaf(false);
				if (leaf) {
					await leaf.setViewState({
						type: BLENDER_BUILDS_VIEW_TYPE,
						active: true,
					});
					loggerDebug(this, 'View state set for new Blender builds view');
				} else {
					loggerWarn(this, 'Failed to get right leaf for Blender builds view');
					return;
				}
			}

			// "Reveal" the leaf in case it is in a collapsed sidebar
			if (leaf) {
				loggerDebug(this, 'Revealing Blender builds view in workspace');
				workspace.revealLeaf(leaf);
				loggerInfo(this, 'Blender builds view opened successfully', { 
					viewType: BLENDER_BUILDS_VIEW_TYPE,
					existingView: leaves.length > 0
				});
			} else {
				loggerError(this, 'No leaf available for Blender builds view');
			}
		} catch (openViewError) {
			loggerError(this, 'Failed to open Blender builds view', { 
				error: openViewError instanceof Error ? openViewError.message : String(openViewError)
			});
		}
	}

	async onunload() {
		loggerDebug(this, 'Beginning plugin unload sequence');
		
		if (this.buildManager) {
			loggerDebug(this, 'Cleaning up build manager resources');
			// Build manager will auto-cleanup via Obsidian's event system
		}

		loggerDebug(this, 'Plugin unload completed - all resources cleaned up');
	}
}
