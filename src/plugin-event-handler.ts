import { MarkdownView, WorkspaceLeaf, TFile, App } from 'obsidian';
import VideoTimestamps from './main'; // Import the main plugin class

export class PluginEventHandler {
    private plugin: VideoTimestamps;
    private app: App;

    constructor(plugin: VideoTimestamps, app: App) {
        this.plugin = plugin;
        this.app = app;
    }

    /**
	 * Handle when the active leaf changes in Obsidian
	 */
	public handleActiveLeafChange(leaf: WorkspaceLeaf | null): void {
        if (leaf && leaf.view instanceof MarkdownView) {
			// Only process markdown views
			this.plugin.detectVideosInActiveView();
		}
	}

    /**
     * Handle when metadata cache changes for a file
     */
    public handleMetadataChange(file: TFile): void {
        const activeView = this.app.workspace.getActiveViewOfType(MarkdownView);
        if (activeView && activeView.file && activeView.file.path === file.path) {
            // Clear the cache and reprocess when file content changes
            if (this.plugin.videoDetector) {
                this.plugin.videoDetector.clearCache();
            }
            this.plugin.detectVideosInActiveView();
        }
    }
}
