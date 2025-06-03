import { ButtonComponent } from 'obsidian';
import { FetchBlenderBuilds } from '../../buildManager';
import type FetchBlenderBuildsPlugin from '../../main';

export class BlenderToolbar {
	private plugin: FetchBlenderBuildsPlugin;
	private buildManager: FetchBlenderBuilds;
	private onRefresh: () => void;
	private onShowSettings: () => void;
	private onToggleFilter: () => void;
	private containerEl: HTMLElement | null = null;
	private buttons: Map<string, ButtonComponent> = new Map();

	constructor(
		plugin: FetchBlenderBuildsPlugin, 
		buildManager: FetchBlenderBuilds,
		onRefresh: () => void,
		onShowSettings: () => void,
		onToggleFilter: () => void
	) {
		this.plugin = plugin;
		this.buildManager = buildManager;
		this.onRefresh = onRefresh;
		this.onShowSettings = onShowSettings;
		this.onToggleFilter = onToggleFilter;
	}
	render(container: HTMLElement): void {
		container.empty();
		this.containerEl = container;
		this.buttons.clear();
		
		// Create toolbar with updated CSS classes
		const toolbarEl = container.createEl('div', { cls: 'blender-toolbar-buttons' });

		// Refresh button
		this.buttons.set('refresh', new ButtonComponent(toolbarEl)
			.setIcon('refresh-cw')
			.setTooltip('Refresh Blender builds')
			.setClass('clickable-icon')
			.onClick(() => this.onRefresh()));

		// Filter button (for future filtering functionality)
		this.buttons.set('filter', new ButtonComponent(toolbarEl)
			.setIcon('filter')
			.setTooltip('Filter builds')
			.setClass('clickable-icon')
			.onClick(() => this.toggleFilter()));
		// Download folder button
		this.buttons.set('folder', new ButtonComponent(toolbarEl)
			.setIcon('folder')
			.setTooltip('Open builds folder')
			.setClass('clickable-icon')
			.onClick(() => this.openBuildsFolder()));

		// Settings button
		this.buttons.set('settings', new ButtonComponent(toolbarEl)
			.setIcon('settings')
			.setTooltip('Plugin settings')
			.setClass('clickable-icon')
			.onClick(() => this.onShowSettings()));
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
	 * Check if a button is active
	 */
	private isButtonActive(buttonKey: string): boolean {
		const button = this.buttons.get(buttonKey);
		return button?.buttonEl?.hasClass('is-active') || false;
	}	/**
	 * Open builds folder
	 */
	private async openBuildsFolder(): Promise<void> {
		try {
			const { exec } = require('child_process');
			const fs = require('fs');
			
			// Try to open the actual builds folder first (where extracted builds are stored)
			const extractsPath = this.buildManager.getExtractsPath();
			const basePath = this.buildManager.getBuildsPath();
			
			let pathToOpen = extractsPath;
			
			// Check if the builds folder exists
			if (!fs.existsSync(extractsPath)) {
				// If builds folder doesn't exist, check if base .blender folder exists
				if (!fs.existsSync(basePath)) {
					// Create the base .blender folder if it doesn't exist
					fs.mkdirSync(basePath, { recursive: true });
					console.log('Created base folder:', basePath);
				}
				// Use the base folder since builds folder doesn't exist yet
				pathToOpen = basePath;
			}
			
			console.log('Opening builds folder:', pathToOpen);
			
			// Open folder in system file manager
			if (process.platform === 'win32') {
				exec(`explorer "${pathToOpen}"`);
			} else if (process.platform === 'darwin') {
				exec(`open "${pathToOpen}"`);
			} else {
				exec(`xdg-open "${pathToOpen}"`);
			}
		} catch (error) {
			console.error('Failed to open builds folder:', error);
		}
	}
}
