import { MarkdownView, Plugin } from 'obsidian';
import { VideoWithTimestamp } from '../video';
import { VideoTimestampsSettings } from '../settings';
import { TimestampHandler } from './types';
import { VideoEventHandler } from './video-event-handler';

/**
 * Manages timestamp restrictions for videos in Obsidian
 */
export class TimestampManager {
    private settings: VideoTimestampsSettings;
    private plugin: Plugin;
    private videoHandler: TimestampHandler;
    
    constructor(settings: VideoTimestampsSettings, plugin: Plugin) {
        this.settings = settings;
        this.plugin = plugin;
        this.videoHandler = new VideoEventHandler();
    }
    
    /**
     * Apply timestamp restrictions to videos in the current view
     */
    public applyTimestampRestrictions(videos: VideoWithTimestamp[]): void {
        const videosWithTimestamps = videos.filter(v => v.timestamp !== null);
        if (videosWithTimestamps.length === 0) {
            return;
        }

        const videoElements = document.querySelectorAll('video');
        const videosByPath = this.groupVideosByPath(videosWithTimestamps);
        const elementsBySource = this.groupElementsBySource(videoElements);
        const isReadingMode = this.checkIfReadingMode();
        
        const processedVideos = new Set<HTMLVideoElement>();

        // Apply timestamp restrictions using three strategies in order of priority
        this.applyDirectDomExtraction(videoElements, processedVideos);
        this.applyMetadataMatching(videosByPath, elementsBySource, isReadingMode, processedVideos);
        this.applyFallbackMatching(videoElements, processedVideos, videosByPath);
    }
    
    /**
     * Clean up all timestamp handlers from a video element
     */
    public cleanupHandlers(videoEl: HTMLVideoElement): void {
        this.videoHandler.cleanup(videoEl);
    }
    
    /**
     * Set up an observer for detecting new videos
     */
    public setupVideoObserver(detectVideosCallback: () => void): MutationObserver {
        const observer = new MutationObserver((mutations) => {
            let videoAdded = false;
            for (const mutation of mutations) {
                if (mutation.type === 'childList') {
                    for (const node of Array.from(mutation.addedNodes)) {
                        if (node instanceof HTMLVideoElement || (node instanceof Element && node.querySelector('video'))) {
                            videoAdded = true;
                            break;
                        }
                    }
                }
                if (videoAdded) break;
            }
            if (videoAdded) {
                setTimeout(() => detectVideosCallback(), 100); // Reduced delay for faster response
            }
        });
        observer.observe(document.body, { childList: true, subtree: true });
        return observer;
    }
    
    /**
     * Group videos by their file path
     */
    private groupVideosByPath(videos: VideoWithTimestamp[]): Map<string, VideoWithTimestamp[]> {
        const videosByPath = new Map<string, VideoWithTimestamp[]>();
        for (const video of videos) {
            if (!videosByPath.has(video.path)) {
                videosByPath.set(video.path, []);
            }
            videosByPath.get(video.path)?.push(video);
        }
        return videosByPath;
    }
    
    /**
     * Group video elements by their source URL
     */
    private groupElementsBySource(videoElements: NodeListOf<HTMLVideoElement>): Map<string, HTMLVideoElement[]> {
        const elementsBySource = new Map<string, HTMLVideoElement[]>();
        for (const videoEl of Array.from(videoElements)) {
            const videoSrc = videoEl.src || videoEl.querySelector('source')?.src || '';
            if (!elementsBySource.has(videoSrc)) {
                elementsBySource.set(videoSrc, []);
            }
            elementsBySource.get(videoSrc)?.push(videoEl);
        }
        return elementsBySource;
    }
    
    /**
     * Check if we're in reading mode
     */
    private checkIfReadingMode(): boolean {
        const activeView = this.plugin.app.workspace.getActiveViewOfType(MarkdownView);
        return activeView ? activeView.getMode() === 'preview' : false;
    }
    
    /**
     * Strategy 1: Apply timestamp restrictions based on direct DOM extraction
     */
    private applyDirectDomExtraction(
        videoElements: NodeListOf<HTMLVideoElement>, 
        processedVideos: Set<HTMLVideoElement>
    ): void {
        for (const videoEl of Array.from(videoElements)) {
            if (processedVideos.has(videoEl)) continue;
            
            const { startTime, endTime, path } = this.extractTimestampsFromDom(videoEl);
            
            if (startTime !== undefined) {
                this.videoHandler.apply(
                    videoEl, 
                    startTime, 
                    endTime !== undefined && endTime >= 0 ? endTime : Infinity, 
                    path || "extracted from DOM"
                );
                processedVideos.add(videoEl);
            }
        }
    }
    
    /**
     * Extract timestamps from the DOM
     */
    private extractTimestampsFromDom(videoEl: HTMLVideoElement): { 
        startTime?: number; 
        endTime?: number; 
        path: string 
    } {
        let start: number | undefined;
        let end: number | undefined;
        let path = "";

        const videoSrc = videoEl.src || videoEl.querySelector('source')?.src || '';
        const srcTimeMatch = videoSrc.match(/#t=([0-9:.]+),?([0-9:.]+)?/);
        if (srcTimeMatch && srcTimeMatch[1] !== '0.001') {
            start = this.parseTimeToSeconds(srcTimeMatch[1]);
            end = srcTimeMatch[2] ? this.parseTimeToSeconds(srcTimeMatch[2]) : undefined;
        }

        if (!start) {
            const parentEl = videoEl.closest('.internal-embed.media-embed');
            if (parentEl) {
                const parentElem = parentEl as HTMLElement;
                const altText = parentElem.getAttribute('alt');
                const srcText = parentElem.getAttribute('src');
                if (altText && altText.includes(' > t=')) {
                    const timeMatch = altText.match(/ > t=([0-9:.]+),?([0-9:.]+)?/);
                    if (timeMatch) {
                        start = this.parseTimeToSeconds(timeMatch[1]);
                        end = timeMatch[2] ? this.parseTimeToSeconds(timeMatch[2]) : undefined;
                        const pathMatch = altText.match(/^(.+?) >/);
                        if (pathMatch) path = pathMatch[1];
                    }
                }
                if (!start && srcText) {
                    const timeMatch = srcText.match(/#t=([0-9:.]+),?([0-9:.]+)?/);
                    if (timeMatch) {
                        start = this.parseTimeToSeconds(timeMatch[1]);
                        end = timeMatch[2] ? this.parseTimeToSeconds(timeMatch[2]) : undefined;
                        const pathMatch = srcText.match(/^(.+?)#t=/);
                        if (pathMatch) path = pathMatch[1];
                    }
                }
            }
        }
        
        return { startTime: start, endTime: end, path };
    }
    
    /**
     * Strategy 2: Apply timestamp restrictions based on metadata matching
     */
    private applyMetadataMatching(
        videosByPath: Map<string, VideoWithTimestamp[]>,
        elementsBySource: Map<string, HTMLVideoElement[]>,
        isReadingMode: boolean,
        processedVideos: Set<HTMLVideoElement>
    ): void {
        for (const [path, videoGroup] of videosByPath.entries()) {
            if (videoGroup.length === 0) continue;
            const matchingElements: HTMLVideoElement[] = [];
            
            for (const [src, elements] of elementsBySource.entries()) {
                const filename = path.split('/').pop()?.split('\\').pop();
                if (filename) {
                    elements.forEach(el => {
                        if (!processedVideos.has(el) && (isReadingMode ? src.includes(filename) : src.includes(path))) {
                            matchingElements.push(el);
                        }
                    });
                }
            }

            const maxToProcess = Math.min(matchingElements.length, videoGroup.length);
            for (let i = 0; i < maxToProcess; i++) {
                const videoEl = matchingElements[i];
                const videoData = videoGroup[i];
                if (videoData.timestamp) {
                    const startTimeSeconds = videoData.timestamp.start;
                    const endTimeSeconds = videoData.timestamp?.end !== undefined && videoData.timestamp?.end >= 0 
                        ? videoData.timestamp?.end 
                        : Infinity;
                    this.videoHandler.apply(videoEl, startTimeSeconds, endTimeSeconds, videoData.path);
                    processedVideos.add(videoEl);
                }
            }
        }
    }
    
    /**
     * Strategy 3: Apply fallback matching for unprocessed videos
     */
    private applyFallbackMatching(
        allVideoElements: NodeListOf<HTMLVideoElement>,
        processedVideos: Set<HTMLVideoElement>,
        videosByPath: Map<string, VideoWithTimestamp[]>
    ): void {
        const unprocessedVideoElements = Array.from(allVideoElements).filter(v => !processedVideos.has(v));
        if (unprocessedVideoElements.length === 0) return;

        const allVideoData: VideoWithTimestamp[] = Array.from(videosByPath.values()).flat();
        if (allVideoData.length === 0) return;

        const maxToProcess = Math.min(unprocessedVideoElements.length, allVideoData.length);
        for (let i = 0; i < maxToProcess; i++) {
            const videoEl = unprocessedVideoElements[i];
            const videoData = allVideoData[i];
            if (videoData.timestamp) {
                const startTimeSeconds = videoData.timestamp.start;
                const endTimeSeconds = videoData.timestamp?.end !== undefined && videoData.timestamp?.end >= 0
                    ? videoData.timestamp?.end
                    : Infinity;
                this.videoHandler.apply(videoEl, startTimeSeconds, endTimeSeconds, videoData.path);
                processedVideos.add(videoEl);
            }
        }
    }
    
    /**
     * Parse a time string to seconds
     */
    private parseTimeToSeconds(timeStr: string): number {
        if (!timeStr.includes(':')) {
            return parseFloat(timeStr);
        }
        const parts = timeStr.split(':');
        const minutes = parseInt(parts[0], 10);
        const seconds = parseFloat(parts[1]);
        return minutes * 60 + seconds;
    }
}
