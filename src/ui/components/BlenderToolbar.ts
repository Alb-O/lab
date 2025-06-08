import { ButtonComponent } from 'obsidian';
import { FetchBlenderBuilds } from '@/build-manager';
import { BUILDS_FOLDER } from '@/constants';
import type BlenderBuildManagerPlugin from '@/main';
import type { BlenderPluginSettings } from '@/settings';
import { debug, info, warn, error, registerLoggerClass } from '@/utils/obsidian-logger';
import * as path from 'path';

export class BlenderToolbar {
	private plugin: BlenderBuildManagerPlugin;
	private buildManager: FetchBlenderBuilds;
	private onRefresh: () => void;
	private onToggleFilter: () => void;
	private onTogglePin: () => void;
	private containerEl: HTMLElement | null = null;
	private buttons: Map<string, ButtonComponent> = new Map();
	constructor(
		plugin: BlenderBuildManagerPlugin,
		buildManager: FetchBlenderBuilds,
		onRefresh: () => void,
		onToggleFilter: () => void,
		onTogglePin: () => void
	) {
		registerLoggerClass(this, 'BlenderToolbar');
		this.plugin = plugin;
		this.buildManager = buildManager;
		this.onRefresh = onRefresh;
		this.onToggleFilter = onToggleFilter;
		this.onTogglePin = onTogglePin;
		debug(this, 'BlenderToolbar initialized');
	}
	render(container: HTMLElement): void {
		debug(this, 'Rendering toolbar');
		container.empty();
		this.containerEl = container;
		this.buttons.clear();
		
		// Create toolbar with updated CSS classes
		const toolbarEl = container.createEl('div', { cls: 'nav-buttons-container' });

		// Refresh button
		this.buttons.set('refresh', new ButtonComponent(toolbarEl)
			.setIcon('refresh-cw')
			.setTooltip('Refresh available builds')
			.setClass('clickable-icon')
			.onClick(() => this.onRefresh()));
		// Filter button (for future filtering functionality)
		this.buttons.set('filter', new ButtonComponent(toolbarEl)
			.setIcon('filter')
			.setTooltip('Filter builds')
			.setClass('clickable-icon')
			.onClick(() => this.toggleFilter()));
		// Pin button (pin symlinked build to top)
		this.buttons.set('pin', new ButtonComponent(toolbarEl)
			.setIcon('pin')
			.setTooltip('Pin symlinked build to top')
			.setClass('clickable-icon')
			.onClick(() => this.onTogglePin()));
		// Download folder button
		this.buttons.set('folder', new ButtonComponent(toolbarEl)
			.setIcon('folder')
			.setTooltip('Open builds folder')
			.setClass('clickable-icon')
			.onClick(() => this.openBuildsFolder()));
	}

	/**
	 * Set a button's active state by its key
	 */
	setButtonActive(buttonKey: string, isActive: boolean): void {
		const button = this.buttons.get(buttonKey);
		if (button && button.buttonEl) {
			if (isActive) {
				button.buttonEl.addClass('is-active');
			} else {
				button.buttonEl.removeClass('is-active');
			}
		}
	}

	/**
	 * Enable or disable specific buttons
	 */
	setButtonsDisabled(states: Record<string, boolean>): void {
		Object.entries(states).forEach(([buttonKey, disabled]) => {
			const button = this.buttons.get(buttonKey);
			if (button) {
				button.setDisabled(disabled);
			}
		});
	}

	/**
	 * Set refreshing state for the refresh button
	 */
	setRefreshingState(isRefreshing: boolean): void {
		const refreshButton = this.buttons.get('refresh');
		if (refreshButton) {
			if (isRefreshing) {
				refreshButton.setDisabled(true);
				// Add a visual indicator that it's refreshing (but keep the icon)
				refreshButton.buttonEl.addClass('is-loading');
			} else {
				refreshButton.setDisabled(false);
				refreshButton.buttonEl.removeClass('is-loading');
			}
		}
	}

	/**
	 * Toggle filter functionality (placeholder)
	 */
	private toggleFilter(): void {
		this.onToggleFilter();
	}

	/**
	 * Open builds folder
	 */
	private async openBuildsFolder(): Promise<void> {
		try {
			const fs = require('fs');
			
			// Try to open the actual builds folder first (where extracted builds are stored)
			const extractsPath = this.buildManager.getExtractsPath();
			const basePath = this.buildManager.getBuildsPath();
			
			// Check if the builds folder exists
			if (!fs.existsSync(extractsPath)) {
				// If builds folder doesn't exist, check if base .blender folder exists
				if (!fs.existsSync(basePath)) {
					// Create the base .blender folder if it doesn't exist
					fs.mkdirSync(basePath, { recursive: true });
				}
			}
			// openWithDefaultApp() requires vault-relative path
		 	// @ts-ignore
			this.plugin.app.openWithDefaultApp(path.join(this.plugin.settings.libraryFolder, BUILDS_FOLDER));
		} catch (error) {
			console.error('Failed to open builds folder:', error);
		}
	}

	/**
	 * Update pin button tooltip based on current state
	 */
	updatePinButtonTooltip(isActive: boolean): void {
		const pinButton = this.buttons.get('pin');
		if (pinButton) {
			const tooltip = isActive 
				? 'Unpin symlinked build from top' 
				: 'Pin symlinked build to top';
			pinButton.setTooltip(tooltip);
			pinButton.setIcon('pin');
		}
	}

	/**
	 * Update toolbar state from plugin settings
	 */
	updateFromSettings(settings: BlenderPluginSettings): void {
		// Update pin button state
		this.setButtonActive('pin', settings.pinSymlinkedBuild);
		this.updatePinButtonTooltip(settings.pinSymlinkedBuild);
	}
}
