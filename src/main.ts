import { MarkdownView, Notice, Plugin } from 'obsidian';
import { DEFAULT_SETTINGS, IVideoTimestampsPlugin, VideoTimestampsSettings, VideoTimestampsSettingTab } from './settings';
import { VideoWithTimestamp, VideoDetector, setupVideoControls } from './video';
import { setupVideoContextMenu, cleanupVideoContextMenu } from './context-menu';
import { TimestampManager } from './timestamps';
import { PluginEventHandler } from './plugin-event-handler';
import * as debug from 'debug';

export default class VideoTimestamps extends Plugin implements IVideoTimestampsPlugin {
	settings: VideoTimestampsSettings;
	videoDetector: VideoDetector;
	timestampController: TimestampManager;
	pluginEventHandler: PluginEventHandler;
	private videoObserver: MutationObserver | null = null;
	private contextMenuCleanup: (() => void) | null = null;

	async onload() {
		// Load settings
		this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());

		// Initialize components
		this.videoDetector = new VideoDetector();

		this.timestampController = new TimestampManager(this.settings, this);
		this.pluginEventHandler = new PluginEventHandler(this, this.app);
		
		// Add a ribbon icon to manually trigger video detection
		const ribbonIconEl = this.addRibbonIcon('video', 'Detect Videos', (evt: MouseEvent) => {
			this.detectVideosInActiveView();
			new Notice('Video detection complete');
		});
		ribbonIconEl.addClass('video-timestamps-ribbon');
		
		// Setup video hover controls
		setupVideoControls();
		
		// Clean up any existing context menu handlers first (in case of reload)
		cleanupVideoContextMenu();
		
		// Setup Obsidian-native context menu for videos
		this.contextMenuCleanup = setupVideoContextMenu(this.app);
		
		// Register for plugin cleanup on unload
		this.register(() => {
			if (this.contextMenuCleanup) {
				this.contextMenuCleanup();
				this.contextMenuCleanup = null;
			}
			// Also directly clean up any remaining context menu handlers
			cleanupVideoContextMenu();
		});

		// Register for file changes to update video detection
		this.registerEvent(
			this.app.workspace.on('active-leaf-change', (leaf) => {
				this.pluginEventHandler.handleActiveLeafChange(leaf);
			})
		);
		
		// Register for file content changes
		this.registerEvent(
			this.app.metadataCache.on('changed', (file) => {
				this.pluginEventHandler.handleMetadataChange(file);
			})
		);
		
		// Add a command to detect videos in current view
		this.addCommand({
			id: 'detect-videos-in-current-view',
			name: 'Detect videos in current view',
			callback: () => {
				const videos = this.detectVideosInActiveView();
				new Notice(`Detected ${videos.length} video${videos.length !== 1 ? 's' : ''}`);
			}
		});
		
		// Set up MutationObserver to watch for dynamically added videos
		this.videoObserver = this.timestampController.setupVideoObserver(() => this.detectVideosInActiveView());
		this.register(() => {
			if (this.videoObserver) {
				this.videoObserver.disconnect();
			}
		});
		
		// Add a settings tab
		this.addSettingTab(new VideoTimestampsSettingTab(this.app, this));
		
		// Initial detection on load
		this.detectVideosInActiveView();
	}
	
	public onunload() {
		// Clean up context menu handlers
		if (this.contextMenuCleanup) {
			this.contextMenuCleanup();
			this.contextMenuCleanup = null;
		}
		cleanupVideoContextMenu();
		
		// Other cleanup is handled by register() calls in onload
		debug.log('[Main] Plugin unloaded, context menu cleaned up');
	}

	/**
	 * Detect videos in all open markdown views
	 * @returns Array of detected videos with timestamps across all views
	 */
	public detectVideosInActiveView(): VideoWithTimestamp[] {
		debug.log('[Main] detectVideosInActiveView called');
		const markdownViews: MarkdownView[] = [];
		this.app.workspace.iterateAllLeaves(leaf => {
			if (leaf.view instanceof MarkdownView) {
				markdownViews.push(leaf.view);
			}
		});
		
		debug.log(`[Main] Found ${markdownViews.length} markdown views`);
		
		if (markdownViews.length === 0) {
			debug.log('[Main] No markdown views found');
			return [];
		}
		
		const allVideos: VideoWithTimestamp[] = [];
		for (const view of markdownViews) {
			const videos = this.videoDetector.getVideosFromActiveView(view);
			debug.log(`[Main] Detected ${videos.length} videos in view: ${view.file?.path}`);
			allVideos.push(...videos);
		}
		
		debug.log('[Main] Total videos detected across all views:', allVideos.length, allVideos);

		this.timestampController.applyTimestampRestrictions(allVideos);
		
		if (allVideos.length > 0) this.videoDetector.debugVideos(allVideos);
		
		return allVideos;
	}

	async saveSettings() {
		await this.saveData(this.settings);
	}
}
