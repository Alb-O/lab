import { MarkdownView } from 'obsidian';
import { extractVideosFromMarkdownView, VideoWithFragment } from './index';

/**
 * Class that handles detecting videos in Markdown views
 */
export class VideoDetector {
    private lastProcessedView: MarkdownView | null = null;
    private lastVideos: VideoWithFragment[] = [];
    
    /**
     * Get videos from the currently active view
     * @param activeView The current markdown view
     * @returns Array of detected videos with their fragments
     */
    public getVideosFromActiveView(activeView: MarkdownView | null): VideoWithFragment[] {
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
     * Debug method to log detected videos
     * Only logs in development environment
     */
    public debugVideos(videos: VideoWithFragment[]): void {
        if (process.env.NODE_ENV === 'production') {
            return; // Don't log anything in production
        }
        
        if (videos.length === 0) {
            console.debug('No videos detected in current view');
            return;
        }
        
        console.debug('Detected videos:', videos.length);
        videos.forEach((video, index) => {
            console.debug(`Video ${index + 1}:`);
            console.debug(`  Path: ${video.path}`);
            console.debug(`  Embedded: ${video.isEmbedded}`);
            if (video.fragment) {
                let startStr = '';
                let endStr = '';
                if (typeof video.fragment.start === 'number') {
                    startStr = video.fragment.start + 's';
                } else if (video.fragment.start && typeof video.fragment.start === 'object' && 'percent' in video.fragment.start) {
                    startStr = video.fragment.start.percent + '%';
                } else {
                    startStr = 'N/A';
                }
                if (typeof video.fragment.end === 'number') {
                    endStr = video.fragment.end === -1 ? 'N/A' : video.fragment.end + 's';
                } else if (video.fragment.end && typeof video.fragment.end === 'object' && 'percent' in video.fragment.end) {
                    endStr = video.fragment.end.percent + '%';
                } else {
                    endStr = 'N/A';
                }
                console.debug(`  Fragment: start=${startStr}, end=${endStr}`);
            } else {
                console.debug('  No fragment');
            }
            console.debug(`  Position: line ${video.position.start.line}`);
        });
    }
}
