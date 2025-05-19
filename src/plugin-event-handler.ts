import { MarkdownView, WorkspaceLeaf, TFile, App } from 'obsidian';
import VideoFragments from './main';
import { updateTimelineStyles } from './video/styles';
// Import the custom declarations to make TypeScript aware of the augmented types
import './custom.d';

/**
 * Handles plugin events for VideoFragments plugin
 */
export class PluginEventHandler {
    private plugin: VideoFragments;
    private app: App;

    constructor(plugin: VideoFragments, app: App) {
        this.plugin = plugin;
        this.app = app;
    }

    /**
     * Handle when the active leaf changes in Obsidian
     */
    public handleActiveLeafChange(leaf: WorkspaceLeaf | null): void {
        if (leaf?.view instanceof MarkdownView) this.plugin.videoDetector?.clearCache();
    }

    /**
     * Handle when metadata cache changes for a file
     */
    public handleMetadataChange(file: TFile): void {
        const activeView = this.app.workspace.getActiveViewOfType(MarkdownView);
        if (activeView?.file?.path === file.path) {
            // Clear the cache and reprocess when file content changes
            this.plugin.videoDetector?.clearCache();
            this.plugin.detectVideosInAllDocuments();
        }
    }

    /**
     * Handle window or workspace resize: update timeline styles for all videos in specified documents.
     */
    public handleResize(targetDocuments: Document[]): void {
        // Helper for percent object
        const isPercentObject = (val: any): val is { percent: number } => {
            return val && typeof val === 'object' && 'percent' in val && typeof val.percent === 'number';
        };

        for (const doc of targetDocuments) {
            doc.querySelectorAll('video').forEach((videoEl) => {
                const state = videoEl._fragmentState;
                let start = state?.startTime;
                let end = state?.endTime;
                const duration = (videoEl as HTMLVideoElement).duration;
                
                if (isPercentObject(start)) {
                    start = duration ? duration * (start.percent / 100) : 0;
                }
                if (isPercentObject(end)) {
                    end = duration ? duration * (end.percent / 100) : Infinity;
                }
                
                if (typeof start === 'number' && typeof end === 'number') {
                    updateTimelineStyles(videoEl as HTMLVideoElement, start, end, duration);
                }
            });
        }
    }

    /**
     * Patch WorkspaceLeaf.onResize to also update timeline styles.
     * Returns the original onResize function for later restoration.
     */
    public patchWorkspaceLeafOnResize(): ((...args: any[]) => any) | null {
        const self = this;
        let foundLeaf: WorkspaceLeaf | undefined = undefined;
        
        this.app.workspace.iterateAllLeaves((leaf: WorkspaceLeaf) => {
            if (!foundLeaf) foundLeaf = leaf;
        });
        
        const proto = (foundLeaf ? Object.getPrototypeOf(foundLeaf) : WorkspaceLeaf.prototype) as any; // Use 'any' for prototype
        
        if (proto?.onResize && !proto._videoTsPatched) {
            const orig = proto.onResize;
            proto.onResize = function (...args: any[]) {
                const result = orig.apply(this, args);
                self.handleResize([document]);
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
        let foundLeaf: WorkspaceLeaf | undefined = undefined;
        
        this.app.workspace.iterateAllLeaves((leaf: WorkspaceLeaf) => {
            if (!foundLeaf) foundLeaf = leaf;
        });
        
        const proto = (foundLeaf ? Object.getPrototypeOf(foundLeaf) : WorkspaceLeaf.prototype) as any; // Use 'any' for prototype
        
        if (proto?._videoTsPatched && orig) {
            proto.onResize = orig;
            delete proto._videoTsPatched;
        }
    }
}
