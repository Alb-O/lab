import { MarkdownView } from 'obsidian';
import { markdownExtractor, VideoWithFragment } from '@markdown';
import { loggerDebug } from '@utils';

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
			loggerDebug(this, 'No active view or file, returning empty videos array');
			return [];
		}
		
		// Return cached results if view hasn't changed
		if (this.lastProcessedView === activeView) {
			loggerDebug(this, `Returning cached videos for view: ${activeView.file.path} (${this.lastVideos.length} videos)`);
			return this.lastVideos;
		}
		
		loggerDebug(this, `Processing new view: ${activeView.file.path}`);
		// Extract and cache results via shared markdownExtractor
		const videos = markdownExtractor.extract(activeView);
		this.lastProcessedView = activeView;
		this.lastVideos = videos;
		
		loggerDebug(this, `Found ${videos.length} videos in view: ${activeView.file.path}`);
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
			loggerDebug(this, 'No videos detected in current view');
			return;
		}
		
		loggerDebug(this, `Detected ${videos.length} videos:`);
		
		videos.forEach((video, index) => {
			loggerDebug(this, `Video ${index + 1}:`);
			loggerDebug(this, `  Path: ${video.path}`);
			loggerDebug(this, `  Embedded: ${video.isEmbedded}`);
			
			if (video.fragment) {
				loggerDebug(this, `  Fragment: ${this.formatFragmentForDebug(video.fragment)}`);
			} else {
				loggerDebug(this, '  No fragment');
			}
			
			loggerDebug(this, `  Position: line ${video.position.start.line}`);
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
