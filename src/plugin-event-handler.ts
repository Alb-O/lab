import { MarkdownView, WorkspaceLeaf, TFile, App } from 'obsidian';
import VideoTimestamps from './main';
import { updateTimelineStyles } from './video/styles';

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

	/**
	 * Handle window or workspace resize: update timeline styles for all videos.
	 */
	public handleResize(): void {
		document.querySelectorAll('video').forEach((videoEl) => {
			const state = (videoEl as any)._timestampState;
			if (state && typeof state.startTime === 'number' && typeof state.endTime === 'number') {
				updateTimelineStyles(
					videoEl as HTMLVideoElement,
					state.startTime,
					state.endTime,
					(videoEl as HTMLVideoElement).duration
				);
			}
		});
	}

	/**
	 * Patch WorkspaceLeaf.onResize to also update timeline styles.
	 * Returns the original onResize function for later restoration.
	 */
	public patchWorkspaceLeafOnResize(): ((...args: any[]) => any) | null {
		const self = this;
		let proto: any = null;
		let foundLeaf: WorkspaceLeaf | undefined = undefined;
		this.app.workspace.iterateAllLeaves((leaf: WorkspaceLeaf) => {
			if (!foundLeaf) foundLeaf = leaf;
		});
		proto = (foundLeaf ? Object.getPrototypeOf(foundLeaf) : WorkspaceLeaf.prototype);
		if (proto && proto.onResize && !proto._videoTsPatched) {
			const orig = proto.onResize;
			proto.onResize = function (...args: any[]) {
				const result = orig.apply(this, args);
				self.handleResize();
				return result;
			};
			proto._videoTsPatched = true;
			return orig;
		}
		return null;
	}

	/**
	 * Restore the original WorkspaceLeaf.onResize if it was patched.
	 */
	public unpatchWorkspaceLeafOnResize(orig: ((...args: any[]) => any) | null): void {
		let proto: any = null;
		let foundLeaf: WorkspaceLeaf | undefined = undefined;
		this.app.workspace.iterateAllLeaves((leaf: WorkspaceLeaf) => {
			if (!foundLeaf) foundLeaf = leaf;
		});
		proto = (foundLeaf ? Object.getPrototypeOf(foundLeaf) : WorkspaceLeaf.prototype);
		if (proto && proto._videoTsPatched && orig) {
			proto.onResize = orig;
			delete proto._videoTsPatched;
		}
	}
}
