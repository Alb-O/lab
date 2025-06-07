import { MarkdownView, Plugin } from 'obsidian';
import { DEFAULT_SETTINGS, IFragmentsPlugin, FragmentsSettings, VideoFragmentsSettingTab } from '@settings';
import { VideoDetector, setupVideoControls } from '@observer';
import { VideoContextMenu } from '@context-menu';
import { FragmentManager } from '@fragments';
import { PluginEventHandler } from './plugin-event-handler';
import { initPluginContext } from 'obsidian-dev-utils/obsidian/Plugin/PluginContext';
import type { VideoWithFragment } from '@markdown';
import { fragmentsDebug, fragmentsInfo } from '@utils';

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
		fragmentsInfo('Plugin onload started');
		initPluginContext(this.app, this.manifest.id);

		// Load settings
		this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());
		fragmentsDebug('Settings loaded:', this.settings);

		// Initialize components
		this.videoDetector = new VideoDetector();
		this.fragmentController = new FragmentManager(this.settings);
		this.pluginEventHandler = new PluginEventHandler(this, this.app);
		fragmentsDebug('Components initialized');

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
			fragmentsDebug('Layout ready, starting initial video detection');
			setTimeout(() => {
				this.detectVideosInAllDocuments();
				this.resizeHandler?.(); 
                this.setupObserversForAllDocuments();
			}, 500); 
		});
		
		fragmentsInfo('Plugin onload completed');
	}

	private registerEvents() {
		fragmentsDebug('Registering plugin events');
		// Register for file changes to update video detection
		this.registerEvent(
			this.app.workspace.on('active-leaf-change', (leaf) => {
				fragmentsDebug('Active leaf changed');
				this.pluginEventHandler.handleActiveLeafChange(leaf);
                this.detectVideosInAllDocuments(); 
			})
		);

		// Register for file content changes
		this.registerEvent(
			this.app.metadataCache.on('changed', (file) => {
				fragmentsDebug('Metadata changed for file:', file?.path);
				this.pluginEventHandler.handleMetadataChange(file);
                this.detectVideosInAllDocuments(); 
			})
		);
		
		// Listen for layout changes
        this.registerEvent(this.app.workspace.on('layout-change', () => {
            fragmentsDebug('Layout changed');
            this.handleLayoutChange();
        }));
	}
	
	private setupUIComponents() {
		fragmentsDebug('Setting up UI components');
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
		fragmentsInfo('Plugin unloading');
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

		fragmentsInfo('Plugin unload completed');
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
	 */	public detectVideosInAllDocuments(): VideoWithFragment[] {
		fragmentsDebug('Starting video detection in all documents');
        const allDocuments = this.getAllRelevantDocuments();
		
		// Get all markdown views from workspace
		const markdownViews: MarkdownView[] = [];
		this.app.workspace.iterateAllLeaves(leaf => {
			if (leaf.view instanceof MarkdownView) {
				markdownViews.push(leaf.view);
			}
		});
		fragmentsDebug(`Found ${markdownViews.length} markdown views to scan`);

		// Collect videos from all markdown views
		const allVideos: VideoWithFragment[] = [];
		markdownViews.forEach(view => {
			const videos = this.videoDetector.getVideosFromActiveView(view);
			allVideos.push(...videos);
		});
		
		fragmentsDebug(`Detected ${allVideos.length} videos total`);

		// Apply fragment restrictions to all detected videos
		this.fragmentController.applyFragmentRestrictions(allVideos, allDocuments);

		// Debug information in development only
		if (allVideos.length > 0 && process.env.NODE_ENV !== 'production') {
			this.videoDetector.debugVideos(allVideos);
		}

		return allVideos;
	}
	
	async saveSettings() {
		fragmentsDebug('Saving settings');
		await this.saveData(this.settings);
	}
}
