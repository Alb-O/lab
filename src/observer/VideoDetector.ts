import { MarkdownView } from 'obsidian';
import { extractVideosFromMarkdownView, VideoWithFragment } from '@markdown';

/**
 * Class that handles detecting videos in Markdown views
 */
export class VideoDetector {
    private lastProcessedView: MarkdownView | null = null;
    private lastVideos: VideoWithFragment[] = [];
    
    /**
     * Get videos from the currently active view
     * Uses caching to avoid re-processing the same view multiple times
     */
    public getVideosFromActiveView(activeView: MarkdownView | null): VideoWithFragment[] {
        if (!activeView?.file) {
            return [];
        }
        
        // Return cached results if view hasn't changed
        if (this.lastProcessedView === activeView) {
            return this.lastVideos;
        }
        
        // Extract and cache results
        const videos = extractVideosFromMarkdownView(activeView);
        this.lastProcessedView = activeView;
        this.lastVideos = videos;
        
        return videos;
    }
    
    /**
     * Clear the cache to force refreshing on next check
     */
    public clearCache(): void {
        this.lastProcessedView = null;
        this.lastVideos = [];
    }
    
    /**
     * Debug method to log detected videos
     * Only logs in development environment
     */
    public debugVideos(videos: VideoWithFragment[]): void {
        if (process.env.NODE_ENV === 'production') {
            return;
        }
        
        if (videos.length === 0) {
            console.debug('No videos detected in current view');
            return;
        }
        
        console.debug(`Detected ${videos.length} videos:`);
        
        videos.forEach((video, index) => {
            console.debug(`Video ${index + 1}:`);
            console.debug(`  Path: ${video.path}`);
            console.debug(`  Embedded: ${video.isEmbedded}`);
            
            if (video.fragment) {
                console.debug(`  Fragment: ${this.formatFragmentForDebug(video.fragment)}`);
            } else {
                console.debug('  No fragment');
            }
            
            console.debug(`  Position: line ${video.position.start.line}`);
        });
    }
    
    /**
     * Format fragment information for debug output
     */
    private formatFragmentForDebug(fragment: { start: any, end: any }): string {
        let startStr = this.formatTimeValueForDebug(fragment.start);
        let endStr = this.formatTimeValueForDebug(fragment.end);
        
        return `start=${startStr}, end=${endStr}`;
    }
    
    /**
     * Format a time value (number or percent object) for debug output
     */
    private formatTimeValueForDebug(value: any): string {
        if (typeof value === 'number') {
            return value === -1 ? 'N/A' : `${value}s`;
        } else if (value && typeof value === 'object' && 'percent' in value) {
            return `${value.percent}%`;
        }
        return 'N/A';
    }
}
