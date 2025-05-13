import { MarkdownView, WorkspaceLeaf, TFile } from 'obsidian';
import VideoTimestamps from './main';

export class PluginEventHandler {
    private plugin: VideoTimestamps;
    private app: any;

    constructor(plugin: VideoTimestamps, app: any) {
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
