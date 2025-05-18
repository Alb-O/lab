import { MarkdownView, Plugin, WorkspaceLeaf } from 'obsidian';
import { DEFAULT_SETTINGS, IVideoTimestampsPlugin, VideoTimestampsSettings, VideoTimestampsSettingTab } from './settings';
import { VideoWithTimestamp, VideoDetector } from './video'; // Corrected import
import { setupVideoControls } from './video/controls'; // Corrected import path
import { setupVideoContextMenu, cleanupVideoContextMenu } from './context-menu';
import { TimestampManager } from './timestamps';
import { PluginEventHandler } from './plugin-event-handler';

export default class VideoTimestamps extends Plugin implements IVideoTimestampsPlugin {
	settings: VideoTimestampsSettings;
	videoDetector: VideoDetector;
	timestampController: TimestampManager;
	pluginEventHandler: PluginEventHandler;
	private videoObservers: MutationObserver[] = [];
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
		setupVideoControls(this.getAllRelevantDocuments.bind(this)); 

		// Clean up any existing context menu handlers first (in case of reload)
		cleanupVideoContextMenu(this.getAllRelevantDocuments()); 
		// Setup Obsidian-native context menu for videos
		this.contextMenuCleanup = setupVideoContextMenu(this, this.settings, this.getAllRelevantDocuments.bind(this)); 

		// Register for plugin cleanup on unload
		this.register(() => {
			if (this.contextMenuCleanup) {
				this.contextMenuCleanup();
				this.contextMenuCleanup = null;
			}
			// Also directly clean up any remaining context menu handlers
			cleanupVideoContextMenu(this.getAllRelevantDocuments());
		});
		// Register for file changes to update video detection
		this.registerEvent(
			this.app.workspace.on('active-leaf-change', (leaf) => {
				this.pluginEventHandler.handleActiveLeafChange(leaf);
                this.detectVideosInAllDocuments(); 
			})
		);

		// Register for file content changes
		this.registerEvent(
			this.app.metadataCache.on('changed', (file) => {
				this.pluginEventHandler.handleMetadataChange(file);
                this.detectVideosInAllDocuments(); 
			})
		);

		// Add a settings tab
		this.addSettingTab(new VideoTimestampsSettingTab(this.app, this));
		// Add a window resize handler to update timeline styles
		this.resizeHandler = () => {
            const allDocuments = this.getAllRelevantDocuments();
			this.pluginEventHandler.handleResize(allDocuments);
		};
		window.addEventListener('resize', this.resizeHandler);
        // Also listen to resize on popout windows
        this.app.workspace.on("window-open", (win) => {
            win.win.addEventListener("resize", this.resizeHandler!);
        });
        this.app.workspace.on("window-close", (win) => {
            win.win.removeEventListener("resize", this.resizeHandler!);
        });


		// Patch WorkspaceLeaf.onResize to also update timeline styles
		this.origLeafOnResize = this.pluginEventHandler.patchWorkspaceLeafOnResize();

		// Initial detection on load, deferred until layout is ready
		this.app.workspace.onLayoutReady(() => {
			setTimeout(() => {
				this.detectVideosInAllDocuments();
				this.resizeHandler?.(); 
                this.setupObserversForAllDocuments();
			}, 500); 
		});

        // Listen for layout changes that might add/remove documents (e.g., opening/closing popouts)
        this.registerEvent(this.app.workspace.on('layout-change', () => {
            // Clean up existing context menu handlers
            if (this.contextMenuCleanup) {
                this.contextMenuCleanup();
                this.contextMenuCleanup = null;
            }
            // Re-setup context menu for all documents
            this.contextMenuCleanup = setupVideoContextMenu(this, this.settings, this.getAllRelevantDocuments.bind(this)); 

            this.setupObserversForAllDocuments();
            this.detectVideosInAllDocuments(); // Re-detect in case new views appeared
            this.resizeHandler?.(); // Update styles for new views
        }));
	}

	public onunload() {
		// Clean up context menu handlers
		if (this.contextMenuCleanup) {
			this.contextMenuCleanup();
			this.contextMenuCleanup = null;
		}
		cleanupVideoContextMenu(this.getAllRelevantDocuments());

		// Clean up resize handler
		if (this.resizeHandler) {
			window.removeEventListener('resize', this.resizeHandler);
            this.app.workspace.iterateAllLeaves(leaf => {
                if (leaf.view.containerEl.ownerDocument !== document && leaf.view.containerEl.ownerDocument?.defaultView) {
                    leaf.view.containerEl.ownerDocument.defaultView.removeEventListener('resize', this.resizeHandler!);
                }
            });
			this.resizeHandler = null;
		}

        // Disconnect all video observers
        this.videoObservers.forEach(observer => observer.disconnect());
        this.videoObservers = [];

		// Restore original WorkspaceLeaf.onResize if patched
		this.pluginEventHandler.unpatchWorkspaceLeafOnResize(this.origLeafOnResize);
		this.origLeafOnResize = null;
	}

    public getAllRelevantDocuments(): Document[] {
        const relevantDocuments: Set<Document> = new Set();
        relevantDocuments.add(document); // Main window's document

        this.app.workspace.iterateAllLeaves(leaf => {
            if (leaf.view && leaf.view.containerEl && leaf.view.containerEl.ownerDocument) {
                relevantDocuments.add(leaf.view.containerEl.ownerDocument);
            }
        });
        return Array.from(relevantDocuments);
    }

    private setupObserversForAllDocuments(): void {
        // Disconnect any existing observers first
        this.videoObservers.forEach(observer => observer.disconnect());
        this.videoObservers = [];

        const documentsToObserve = this.getAllRelevantDocuments();
        documentsToObserve.forEach(doc => {
            // Check if the document body exists before observing
            if (doc.body) {
                const observer = this.timestampController.setupVideoObserver(doc, () => this.detectVideosInAllDocuments());
                this.videoObservers.push(observer);
            }
        });
    }

	/**
	 * Detect videos in all open markdown views across all documents
	 * @returns Array of detected videos with timestamps across all views
	 */
	public detectVideosInAllDocuments(): VideoWithTimestamp[] {
		const markdownViews: MarkdownView[] = [];
        const allDocuments = this.getAllRelevantDocuments();

		this.app.workspace.iterateAllLeaves(leaf => {
			if (leaf.view instanceof MarkdownView) {
				markdownViews.push(leaf.view);
			}
		});

		if (markdownViews.length === 0 && allDocuments.every(doc => doc.querySelectorAll('video').length === 0)) {
            // If no markdown views and no videos in any document, clear restrictions from all potential videos
            allDocuments.forEach(doc => {
                doc.querySelectorAll('video').forEach(videoEl => this.timestampController.cleanupHandlers(videoEl));
            });
			return [];
		}

		const allVideos: VideoWithTimestamp[] = [];
		for (const view of markdownViews) {
			const videos = this.videoDetector.getVideosFromActiveView(view);
			allVideos.push(...videos);
		}

		this.timestampController.applyTimestampRestrictions(allVideos, allDocuments);

		// Only debug in non-production environment
		if (allVideos.length > 0 && process.env.NODE_ENV !== 'production') {
			this.videoDetector.debugVideos(allVideos);
		}

		return allVideos;
	}
	async saveSettings() {
		await this.saveData(this.settings);
	}
}
