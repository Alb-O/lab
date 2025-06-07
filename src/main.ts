import { MarkdownView, Plugin } from 'obsidian';
import { DEFAULT_SETTINGS, IFragmentsPlugin, FragmentsSettings, VideoFragmentsSettingTab } from '@settings';
import { VideoDetector, setupVideoControls } from '@observer';
import { VideoContextMenu } from '@context-menu';
import { FragmentManager } from '@fragments';
import { PluginEventHandler } from './plugin-event-handler';
import { initPluginContext } from 'obsidian-dev-utils/obsidian/Plugin/PluginContext';
import type { VideoWithFragment } from '@markdown';

export default class Fragments extends Plugin implements IFragmentsPlugin {
	settings: FragmentsSettings;
	videoDetector: VideoDetector;
	fragmentController: FragmentManager;
	pluginEventHandler: PluginEventHandler;

	private videoObservers: MutationObserver[] = [];
	private contextMenu: VideoContextMenu | null = null;
	private resizeHandler: (() => void) | null = null;
	private origLeafOnResize: ((...args: any[]) => any) | null = null;

	async onload() {
		initPluginContext(this.app, this.manifest.id);

		// Load settings
		this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());

		// Initialize components
		this.videoDetector = new VideoDetector();
		this.fragmentController = new FragmentManager(this.settings);
		this.pluginEventHandler = new PluginEventHandler(this, this.app);

		// Register event handlers
		this.registerEvents();
		
		// Setup UI components
		this.setupUIComponents();
		
		// Add settings tab
		this.addSettingTab(new VideoFragmentsSettingTab(this.app, this));

		// Setup resize handling
		this.setupResizeHandling();

		// Initial detection on load, deferred until layout is ready
		this.app.workspace.onLayoutReady(() => {
			setTimeout(() => {
				this.detectVideosInAllDocuments();
				this.resizeHandler?.(); 
                this.setupObserversForAllDocuments();
			}, 500); 
		});
	}

	private registerEvents() {
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
		
		// Listen for layout changes
        this.registerEvent(this.app.workspace.on('layout-change', () => {
            this.handleLayoutChange();
        }));
	}
	
	private setupUIComponents() {
		// Setup video hover controls
		setupVideoControls(this.getAllRelevantDocuments.bind(this)); 

		// Setup context menu class
		if (this.contextMenu) this.contextMenu.cleanup();
		this.contextMenu = new VideoContextMenu(this, this.settings, this.getAllRelevantDocuments.bind(this));
		this.contextMenu.setup();

		// Register for plugin cleanup on unload
		this.register(() => {
			if (this.contextMenu) {
				this.contextMenu.cleanup();
				this.contextMenu = null;
			}
		});
	}
	
	private setupResizeHandling() {
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
	}
	
	private handleLayoutChange() {
		// Clean up existing context menu handlers
		if (this.contextMenu) {
			this.contextMenu.cleanup();
			this.contextMenu = null;
		}
		
		// Re-setup context menu for all documents
		this.contextMenu = new VideoContextMenu(
			this, 
			this.settings, 
			this.getAllRelevantDocuments.bind(this)
		); 
		this.contextMenu.setup();

		// Re-setup observers and detect videos
		this.setupObserversForAllDocuments();
		this.detectVideosInAllDocuments();
		this.resizeHandler?.();
	}

	public onunload() {
		// Clean up context menu handlers
		if (this.contextMenu) {
			this.contextMenu.cleanup();
			this.contextMenu = null;
		}

		// Clean up resize handler
		if (this.resizeHandler) {
			window.removeEventListener('resize', this.resizeHandler);
            
			// Remove from popout windows
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
            if (leaf.view?.containerEl?.ownerDocument) {
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
                const observer = this.fragmentController.setupVideoObserver(doc, () => this.detectVideosInAllDocuments());
                this.videoObservers.push(observer);
            }
        });
    }

	/**
	 * Detect videos in all open markdown views across all documents
	 * @returns Array of detected videos with fragments across all views
	 */
	public detectVideosInAllDocuments(): VideoWithFragment[] {
		const markdownViews: MarkdownView[] = [];
        const allDocuments = this.getAllRelevantDocuments();

		this.app.workspace.iterateAllLeaves(leaf => {
			if (leaf.view instanceof MarkdownView) {
				markdownViews.push(leaf.view);
			}
		});

		// If no markdown views and no videos in any document, clean up and return empty array
		if (markdownViews.length === 0 && allDocuments.every(doc => doc.querySelectorAll('video').length === 0)) {
            allDocuments.forEach(doc => {
                doc.querySelectorAll('video').forEach(videoEl => this.fragmentController.cleanupHandlers(videoEl));
            });
			return [];
		}

		// Collect videos from all markdown views
		const allVideos: VideoWithFragment[] = [];
		markdownViews.forEach(view => {
			const videos = this.videoDetector.getVideosFromActiveView(view);
			allVideos.push(...videos);
		});

		// Apply fragment restrictions to all detected videos
		this.fragmentController.applyFragmentRestrictions(allVideos, allDocuments);

		// Debug information in development only
		if (allVideos.length > 0 && process.env.NODE_ENV !== 'production') {
			this.videoDetector.debugVideos(allVideos);
		}

		return allVideos;
	}
	
	async saveSettings() {
		await this.saveData(this.settings);
	}
}
