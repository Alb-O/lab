import { MarkdownView, Plugin } from 'obsidian';
import { DEFAULT_SETTINGS, IVideoTimestampsPlugin, VideoTimestampsSettings, VideoTimestampsSettingTab } from './settings';
import { VideoWithTimestamp, VideoDetector, setupVideoControls } from './video';
import { setupVideoContextMenu, cleanupVideoContextMenu } from './context-menu';
import { TimestampManager } from './timestamps';
import { PluginEventHandler } from './plugin-event-handler';
import { VideoRestrictionHandler } from './video/restriction-handler';

export default class VideoTimestamps extends Plugin implements IVideoTimestampsPlugin {
	settings: VideoTimestampsSettings;
	videoDetector: VideoDetector;
	timestampController: TimestampManager;
	pluginEventHandler: PluginEventHandler;
	private videoObserver: MutationObserver | null = null;
	private contextMenuCleanup: (() => void) | null = null;
	private resizeHandler: (() => void) | null = null;
	private origLeafOnResize: ((...args: any[]) => any) | null = null;

	async onload() {
		// Load settings
		this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());

		// Initialize components
		this.videoDetector = new VideoDetector();

		this.timestampController = new TimestampManager(this.settings);
		this.pluginEventHandler = new PluginEventHandler(this, this.app);

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

		// Set up MutationObserver to watch for dynamically added videos
		this.videoObserver = this.timestampController.setupVideoObserver(() => this.detectVideosInActiveView());
		this.register(() => {
			if (this.videoObserver) {
				this.videoObserver.disconnect();
			}
		});

		// Add a settings tab
		this.addSettingTab(new VideoTimestampsSettingTab(this.app, this));

		// Add a window resize handler to update timeline styles
		this.resizeHandler = () => {
			this.pluginEventHandler.handleResize();
		};
		window.addEventListener('resize', this.resizeHandler);

		// Patch WorkspaceLeaf.onResize to also update timeline styles
		this.origLeafOnResize = this.pluginEventHandler.patchWorkspaceLeafOnResize();

		// Initial detection on load, deferred until layout is ready
		this.app.workspace.onLayoutReady(() => {
			// Add a small delay to allow video elements to fully initialize their dimensions
			setTimeout(() => {
				this.detectVideosInActiveView();
				this.resizeHandler?.(); // Ensure timeline styles are correct after layout
			}, 500); // 500ms delay, can be adjusted
		});
	}

	public onunload() {
		// Clean up context menu handlers
		if (this.contextMenuCleanup) {
			this.contextMenuCleanup();
			this.contextMenuCleanup = null;
		}
		cleanupVideoContextMenu();

		// Clean up resize handler
		if (this.resizeHandler) {
			window.removeEventListener('resize', this.resizeHandler);
			this.resizeHandler = null;
		}

		// Restore original WorkspaceLeaf.onResize if patched
		this.pluginEventHandler.unpatchWorkspaceLeafOnResize(this.origLeafOnResize);
		this.origLeafOnResize = null;
	}

	/**
	 * Detect videos in all open markdown views
	 * @returns Array of detected videos with timestamps across all views
	 */
	public detectVideosInActiveView(): VideoWithTimestamp[] {
		const markdownViews: MarkdownView[] = [];
		this.app.workspace.iterateAllLeaves(leaf => {
			if (leaf.view instanceof MarkdownView) {
				markdownViews.push(leaf.view);
			}
		});


		if (markdownViews.length === 0) {
			return [];
		}

		const allVideos: VideoWithTimestamp[] = [];
		for (const view of markdownViews) {
			const videos = this.videoDetector.getVideosFromActiveView(view);
			allVideos.push(...videos);
		}

		this.timestampController.applyTimestampRestrictions(allVideos);

		if (allVideos.length > 0) this.videoDetector.debugVideos(allVideos);

		return allVideos;
	}

	async saveSettings() {
		await this.saveData(this.settings);
	}

	/**
	 * Reapply timestamp restriction handlers without full plugin reload
	 */
	public reinitializeRestrictionHandlers(): void {
		const handler = new VideoRestrictionHandler();
		const videos = Array.from(document.querySelectorAll('video')) as HTMLVideoElement[];
		videos.forEach(videoEl => {
			const state = (videoEl as any)._timestampState;
			if (state) {
				handler.apply(videoEl, state.startTime, state.endTime, state.path, this.settings, true);
			}
		});
	}
}
