import { MarkdownView } from 'obsidian';
import { extractVideosFromMarkdownView, VideoWithTimestamp } from './index';
import * as debug from 'debug';

/**
 * Class that handles detecting videos in Markdown views
 */
export class VideoDetector {
    private lastProcessedView: MarkdownView | null = null;
    private lastVideos: VideoWithTimestamp[] = [];
    
    /**
     * Get videos from the currently active view
     * @param activeView The current markdown view
     * @returns Array of detected videos with their timestamps
     */
    public getVideosFromActiveView(activeView: MarkdownView | null): VideoWithTimestamp[] {
        if (!activeView || !activeView.file) {
            return [];
        }
        
        // If we've already processed this view and it hasn't changed, return cached results
        if (this.lastProcessedView === activeView) {
            return this.lastVideos;
        }
        
        // Extract videos from the view
        const videos = extractVideosFromMarkdownView(activeView);
        
        // Cache the results
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
     * Debug method to log detected videos to the debug
     */
    public debugVideos(videos: VideoWithTimestamp[]): void {
        if (videos.length === 0) {
            debug.log('No videos detected in current view');
            return;
        }
        
        debug.log('Detected videos:', videos.length);
        videos.forEach((video, index) => {
            debug.log(`Video ${index + 1}:`);
            debug.log(`  Path: ${video.path}`);
            debug.log(`  Embedded: ${video.isEmbedded}`);
            if (video.timestamp) {
                debug.log(`  Timestamp: start=${video.timestamp.start}s, end=${video.timestamp.end === -1 ? 'N/A' : video.timestamp.end + 's'}`);
            } else {
                debug.log('  No timestamp');
            }
            debug.log(`  Position: line ${video.position.start.line}`);
        });
    }
}
