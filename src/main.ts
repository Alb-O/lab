import { MarkdownView, Notice, Plugin, WorkspaceLeaf, TFile } from 'obsidian';
import { VideoDetector } from './video-detector';
import { DEFAULT_SETTINGS, IVideoTimestampsPlugin, VideoTimestampsSettings, VideoTimestampsSettingTab } from './settings';
import { VideoWithTimestamp } from './utils';
import { setupVideoControls } from './video-controls';
import { TimestampController } from './timestamp-controller';
import { StatusBarController } from './status-bar-controller';
import { PluginEventHandler } from './plugin-event-handler';

export default class VideoTimestamps extends Plugin implements IVideoTimestampsPlugin {
	settings: VideoTimestampsSettings;
	videoDetector: VideoDetector;
	statusBarItemEl: HTMLElement;
	timestampController: TimestampController;
	public statusBarController: StatusBarController; // Made public for PluginEventHandler
	pluginEventHandler: PluginEventHandler;
	private videoObserver: MutationObserver | null = null;

	async onload() {
		// Load settings
		this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());

		// Initialize components
		this.videoDetector = new VideoDetector();
		this.statusBarItemEl = this.addStatusBarItem();
		this.statusBarItemEl.setText('No videos detected');
		this.statusBarItemEl.addClass('video-timestamps-status');

		this.timestampController = new TimestampController(this.settings, this);
		this.statusBarController = new StatusBarController(this.statusBarItemEl, this.settings);
		this.pluginEventHandler = new PluginEventHandler(this, this.app);
		
		// Add a ribbon icon to manually trigger video detection
		const ribbonIconEl = this.addRibbonIcon('video', 'Detect Videos', (evt: MouseEvent) => {
			this.detectVideosInActiveView();
			new Notice('Video detection complete');
		});
		ribbonIconEl.addClass('video-timestamps-ribbon');
		
		// Setup video hover controls
		setupVideoControls();

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
		// Clean up any resources or event listeners
		console.log('Video Timestamps plugin unloaded');
		// The MutationObserver is disconnected via this.register in onload
	}

	/**
	 * Detect videos in all open markdown views
	 * @returns Array of detected videos with timestamps across all views
	 */
	public detectVideosInActiveView(): VideoWithTimestamp[] {
		console.log('Debug - detectVideosInActiveView called');
		
		const markdownViews: MarkdownView[] = [];
		this.app.workspace.iterateAllLeaves(leaf => {
			if (leaf.view instanceof MarkdownView) {
				markdownViews.push(leaf.view);
			}
		});
		
		console.log(`Debug - Found ${markdownViews.length} markdown views`);
		
		if (markdownViews.length === 0) {
			console.log('Debug - No markdown views found');
            this.statusBarController.updateStatusBar([]); // Update status bar even if no markdown views
			return [];
		}
		
		const allVideos: VideoWithTimestamp[] = [];
		for (const view of markdownViews) {
			const videos = this.videoDetector.getVideosFromActiveView(view);
			console.log(`Debug - Detected ${videos.length} videos in view: ${view.file?.path}`);
			allVideos.push(...videos);
		}
		
		console.log('Debug - Total videos detected across all views:', allVideos.length, allVideos);
		
		const activeView = this.app.workspace.getActiveViewOfType(MarkdownView);
		if (activeView) {
			const activeViewVideos = allVideos.filter(v => v.file?.path === activeView.file?.path);
			this.statusBarController.updateStatusBar(activeViewVideos);
		} else {
			this.statusBarController.updateStatusBar([]);
		}
		
		this.timestampController.applyTimestampRestrictions(allVideos);
		
		if (this.settings.debugMode && allVideos.length > 0) {
			this.videoDetector.debugVideos(allVideos);
		}
		
		return allVideos;
	}

	async saveSettings() {
		await this.saveData(this.settings);
	}
}
