import { Notice, Plugin, WorkspaceLeaf, addIcon } from 'obsidian';
import { initLogger, registerLoggerClass, debug, info, warn, error } from '@utils/obsidian-logger';
import { BLENDER_ICON_SVG } from '@constants';
import { BlenderPluginSettings, DEFAULT_SETTINGS, BlenderBuildManagerSettingsTab } from '@settings';
import { BuildManager } from '@build-manager';
import { BlenderBuildsView, BLENDER_BUILDS_VIEW_TYPE } from '@views/BlenderBuildsView';

export default class BlenderBuildManagerPlugin extends Plugin {
	settings: BlenderPluginSettings;
	buildManager: BuildManager;	async onload() {
		// Initialize the logger system
		debug(this, 'Initializing logger system');
		initLogger(this);
		registerLoggerClass(this, 'BlenderBuildManagerPlugin');

		info(this, 'Blender Build Manager Plugin starting initialization', { version: this.manifest.version });

		try {
			debug(this, 'Loading plugin settings from storage');
			await this.loadSettings();
			
			// Initialize build manager
			debug(this, 'Creating build manager instance');
			// @ts-ignore - Using Obsidian's internal API
			const vaultPath = this.app.vault.adapter.basePath || this.app.vault.adapter.path || '';
			this.buildManager = new BuildManager(vaultPath, this.settings);
			
			debug(this, 'Registering build manager with logger for enhanced debugging');
			registerLoggerClass(this.buildManager, 'BuildManager');
			
			debug(this, 'Initializing plugin components');
			this.initializeComponents();

			info(this, 'Blender Build Manager Plugin successfully loaded and ready', { 
				settingsLoaded: !!this.settings,
				buildManagerInitialized: !!this.buildManager
			});
		} catch (initError) {
			error(this, 'Failed to initialize Blender Build Manager Plugin', { 
				error: initError instanceof Error ? initError.message : String(initError),
				stack: initError instanceof Error ? initError.stack : undefined
			});
			throw initError;
		}
	}
	private initializeComponents() {
		debug(this, 'Registering Blender builds view type');
		this.registerView(
			BLENDER_BUILDS_VIEW_TYPE,
			(leaf: WorkspaceLeaf) => new BlenderBuildsView(leaf, this, this.buildManager)
		);
		
		debug(this, 'Registering custom Blender icon');
		addIcon('blender-logo', BLENDER_ICON_SVG);

		debug(this, 'Adding ribbon icon for Blender builds');
		this.addRibbonIcon('blender-logo', 'Open Blender build manager', (evt: MouseEvent) => {
			debug(this, 'User clicked ribbon icon', { mouseEvent: evt.type });
			this.handleRibbonClick();
		});

		debug(this, 'Registering command palette commands');
		this.registerCommands();
		
		debug(this, 'Adding settings tab to plugin');
		this.addSettingTab(new BlenderBuildManagerSettingsTab(this.app, this));
	}

	private handleRibbonClick() {
		debug(this, 'Processing ribbon icon click');
		try {
			info(this, 'Ribbon click processed successfully - opening builds view');
			this.openBuildsView();
		} catch (ribbonError) {
			error(this, 'Error handling ribbon click', { 
				error: ribbonError instanceof Error ? ribbonError.message : String(ribbonError) 
			});
		}
	}

	private registerCommands() {
		this.addCommand({
			id: 'open-blender-builds',
			name: 'Open Blender Builds',
			callback: () => {
				info(this, 'User executed open blender builds command');
				this.executeOpenBuildsCommand();
			}
		});

		this.addCommand({
			id: 'refresh-blender-builds',
			name: 'Refresh available builds',
			callback: async () => {
				info(this, 'User executed refresh builds command');
				this.executeRefreshBuildsCommand();
			}
		});

		this.addCommand({
			id: 'debug-build-detection',
			name: 'Debug Build Detection',
			callback: async () => {
				info(this, 'User executed debug build detection command');
				this.executeDebugBuildDetectionCommand();
			}
		});

		this.addCommand({
			id: 'check-build-paths',
			name: 'Check Build Paths',
			callback: () => {
				info(this, 'User executed check build paths command');
				this.executeCheckBuildPathsCommand();
			}
		});
	
		this.addCommand({
			id: 'debug-cache-contents',
			name: 'Debug Cache Contents',
			callback: () => {
				info(this, 'User executed debug cache contents command');
				this.executeDebugCacheContentsCommand();
			}
		});

		this.addCommand({
			id: 'open-blender-directory',
			name: 'Open Blender Directory',
			callback: () => {
				info(this, 'User executed open blender directory command');
				this.executeOpenBlenderDirectoryCommand();
			}
		});

		this.addCommand({
			id: 'migrate-existing-builds',
			name: 'Migrate Existing Builds to Metadata Cache',
			callback: async () => {
				info(this, 'User executed migrate existing builds command');
				this.executeMigrateExistingBuildsCommand();
			}
		});
	}

	private executeOpenBuildsCommand() {
		debug(this, 'Executing open Blender builds command');
		try {
			this.openBuildsView();
			info(this, 'Open builds command executed successfully');
		} catch (commandError) {
			error(this, 'Failed to execute open builds command', { 
				error: commandError instanceof Error ? commandError.message : String(commandError)
			});
		}
	}

	private async executeRefreshBuildsCommand() {
		debug(this, 'Executing refresh Blender builds command');
		new Notice('Refreshing Blender builds...');
		try {
			await this.buildManager.refreshBuilds();
			info(this, 'Refresh builds command executed successfully');
			new Notice('Blender builds refreshed successfully!');
		} catch (commandError) {
			error(this, 'Failed to execute refresh builds command', { 
				error: commandError instanceof Error ? commandError.message : String(commandError)
			});
			new Notice(`Failed to refresh builds: ${commandError.message}`);
		}
	}

	private async executeDebugBuildDetectionCommand() {
		debug(this, 'Executing debug build detection command');
		try {
			new Notice('Starting build detection debug...', 3000);
			await this.buildManager.forceRefreshExtractedBuilds();
			info(this, 'Debug build detection command executed successfully');
		} catch (commandError) {
			error(this, 'Failed to execute debug build detection command', { 
				error: commandError instanceof Error ? commandError.message : String(commandError)
			});
			new Notice(`Failed to debug build detection: ${commandError.message}`, 8000);
			// Keep one console.error for severe debugging issues
			console.error('[DEBUG] Build detection error:', commandError);
		}
	}

	private executeCheckBuildPathsCommand() {
		debug(this, 'Executing check build paths command');
		try {
			const buildsPath = this.buildManager.getBuildsPath();
			const extractsPath = this.buildManager.getExtractsPath();
			
			info(this, 'Check build paths command executed successfully', { 
				buildsPath, 
				extractsPath 
			});
			new Notice(`Builds path: ${buildsPath}`, 8000);
			new Notice(`Extracts path: ${extractsPath}`, 8000);
		} catch (commandError) {
			error(this, 'Failed to execute check build paths command', { 
				error: commandError instanceof Error ? commandError.message : String(commandError)
			});
		}
	}

	private executeDebugCacheContentsCommand() {
		debug(this, 'Executing debug cache contents command');
		try {
			this.buildManager.debugShowCacheContents();
			info(this, 'Debug cache contents command executed successfully');
		} catch (commandError) {
			error(this, 'Failed to execute debug cache contents command', { 
				error: commandError instanceof Error ? commandError.message : String(commandError)
			});
		}
	}

	private executeOpenBlenderDirectoryCommand() {
		debug(this, 'Executing open Blender directory command');
		try {
			this.buildManager.openBuildsDirectory();
			info(this, 'Open Blender directory command executed successfully');
		} catch (commandError) {
			error(this, 'Failed to execute open Blender directory command', { 
				error: commandError instanceof Error ? commandError.message : String(commandError)
			});
		}
	}

	private async executeMigrateExistingBuildsCommand() {
		debug(this, 'Executing migrate existing builds command');
		try {
			await this.buildManager.scanAndMigrateExistingBuilds();
			info(this, 'Migrate existing builds command executed successfully');
			new Notice('Migration scan completed - check console for details');
		} catch (commandError) {
			error(this, 'Failed to execute migrate existing builds command', { 
				error: commandError instanceof Error ? commandError.message : String(commandError)
			});
		}	}

	async loadSettings() {
		debug(this, 'Loading plugin settings from storage');
		this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());
		info(this, 'Plugin settings loaded successfully', { 
			settingsKeys: Object.keys(this.settings),
			libraryFolder: this.settings.libraryFolder 
		});
	}

	async saveSettings() {
		debug(this, 'Attempting to save plugin settings to storage');
		try {
			await this.saveData(this.settings);
			info(this, 'Plugin settings successfully saved to storage', { 
				settingsKeys: Object.keys(this.settings),
				libraryFolder: this.settings.libraryFolder 
			});
			
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
		} catch (saveError) {
			error(this, 'Failed to save plugin settings to storage', { 
				error: saveError instanceof Error ? saveError.message : String(saveError),
				settings: this.settings 
			});
			throw saveError;
		}
	}	async openBuildsView(): Promise<void> {
		debug(this, 'Processing request to open Blender builds view');
		try {
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
				debug(this, 'Creating new Blender builds view in right sidebar');
				leaf = workspace.getRightLeaf(false);
				if (leaf) {
					await leaf.setViewState({
						type: BLENDER_BUILDS_VIEW_TYPE,
						active: true,
					});
					debug(this, 'View state set for new Blender builds view');
				} else {
					warn(this, 'Failed to get right leaf for Blender builds view');
					return;
				}
			}

			// "Reveal" the leaf in case it is in a collapsed sidebar
			if (leaf) {
				debug(this, 'Revealing Blender builds view in workspace');
				workspace.revealLeaf(leaf);
				info(this, 'Blender builds view opened successfully', { 
					viewType: BLENDER_BUILDS_VIEW_TYPE,
					existingView: leaves.length > 0
				});
			} else {
				error(this, 'No leaf available for Blender builds view');
			}
		} catch (openViewError) {
			error(this, 'Failed to open Blender builds view', { 
				error: openViewError instanceof Error ? openViewError.message : String(openViewError)
			});
		}
	}

	async onunload() {
		debug(this, 'Beginning plugin unload sequence');
		
		if (this.buildManager) {
			debug(this, 'Cleaning up build manager resources');
			// Build manager will auto-cleanup via Obsidian's event system
		}

		debug(this, 'Plugin unload completed - all resources cleaned up');
	}
}
