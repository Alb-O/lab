import { ButtonComponent } from 'obsidian';
import { BlenderBuildManager } from '../../buildManager';
import type BlenderBuildManagerPlugin from '../../main';

export class BlenderToolbar {
	private plugin: BlenderBuildManagerPlugin;
	private buildManager: BlenderBuildManager;
	private onRefresh: () => void;
	private onShowSettings: () => void;
	private containerEl: HTMLElement | null = null;
	private buttons: Map<string, ButtonComponent> = new Map();

	constructor(
		plugin: BlenderBuildManagerPlugin, 
		buildManager: BlenderBuildManager,
		onRefresh: () => void,
		onShowSettings: () => void
	) {
		this.plugin = plugin;
		this.buildManager = buildManager;
		this.onRefresh = onRefresh;
		this.onShowSettings = onShowSettings;
	}

	render(container: HTMLElement): void {
		container.empty();
		this.containerEl = container;
		this.buttons.clear();
		
		// Create toolbar with nav-buttons-container class (matches SVN plugin)
		const toolbarEl = container.createEl('div', { cls: 'nav-buttons-container' });

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
			.setTooltip('Open downloads folder')
			.setClass('clickable-icon')
			.onClick(() => this.openDownloadsFolder()));

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
	 * Toggle filter functionality (placeholder)
	 */
	private toggleFilter(): void {
		// TODO: Implement filter toggle
		this.setButtonActive('filter', !this.isButtonActive('filter'));
	}

	/**
	 * Check if a button is active
	 */
	private isButtonActive(buttonKey: string): boolean {
		const button = this.buttons.get(buttonKey);
		return button?.buttonEl?.hasClass('is-active') || false;
	}
	/**
	 * Open downloads folder
	 */
	private async openDownloadsFolder(): Promise<void> {
		try {
			const { exec } = require('child_process');
			const path = require('path');
			const downloadPath = path.join(this.plugin.settings.libraryFolder);
			
			// Open folder in system file manager
			if (process.platform === 'win32') {
				exec(`explorer "${downloadPath}"`);
			} else if (process.platform === 'darwin') {
				exec(`open "${downloadPath}"`);
			} else {
				exec(`xdg-open "${downloadPath}"`);
			}
		} catch (error) {
			console.error('Failed to open downloads folder:', error);
		}
	}
}
