import { Menu, Plugin } from 'obsidian';
import { FragmentsSettings } from '@settings';
import { observeElements } from '@observer';
import { addOpenCommands, addEmbedActionsCommands, addSetCommands, addSystemCommands } from '@context-menu/items';
import { loggerDebug } from '@utils';

export class VideoContextMenu {
	private plugin: Plugin;
	private settings: FragmentsSettings;
	private getAllRelevantDocuments: () => Document[];
	private initializedElements = new WeakSet<HTMLVideoElement>();
	private cleanupFn: (() => void) | null = null;

	constructor(plugin: Plugin, settings: FragmentsSettings, getAllRelevantDocuments: () => Document[]) {
		this.plugin = plugin;
		this.settings = settings;
		this.getAllRelevantDocuments = getAllRelevantDocuments;
	}

	/**
	 * Set up the context menu for all video elements.
	 * Returns a cleanup function.
	 */public setup(): () => void {
		loggerDebug(this, 'Setting up video context menu');
		this.cleanup();
		const initContext = (video: HTMLVideoElement) => {
			if (this.initializedElements.has(video) && video._videoContextMenuHandler) {
				return;
			}
			loggerDebug(this, 'Initializing context menu for video element');
			const contextMenuHandler = (event: MouseEvent) => {
				event.preventDefault();
				loggerDebug(this, 'Context menu opened for video');
				const menu = new Menu();
				addOpenCommands(menu, this.plugin, video);
				addEmbedActionsCommands(menu, this.plugin, this.settings, video);
				addSetCommands(menu, this.plugin, this.settings, video);
				addSystemCommands(menu, this.plugin, video);
				menu.showAtPosition({ x: event.clientX, y: event.clientY });
			};
			video._videoContextMenuHandler = contextMenuHandler;
			video.addEventListener('contextmenu', contextMenuHandler);
			this.initializedElements.add(video);
			video.dataset.contextMenuInitialized = 'true';
		};
		this.cleanupFn = observeElements(this.getAllRelevantDocuments, 'video', initContext);
		return this.cleanupFn;
	}

	/**
	 * Clean up all context menu handlers from all videos.
	 */
	public cleanup(): void {
		loggerDebug(this, 'Cleaning up video context menu');
		this.getAllRelevantDocuments().forEach(doc => {
			doc.querySelectorAll('video').forEach((video: HTMLVideoElement) => {
				if (video._videoContextMenuHandler) {
					video.removeEventListener('contextmenu', video._videoContextMenuHandler);
					delete video._videoContextMenuHandler;
					video.dataset.contextMenuInitialized = 'false';
				}
			});
		});
		if (this.cleanupFn) {      this.cleanupFn();
			this.cleanupFn = null;
		}
		loggerDebug(this, 'Video context menu cleanup completed');
	}
}
