import { MarkdownView, Plugin } from 'obsidian';
import { VideoWithTimestamp } from '../video';
import { VideoTimestampsSettings } from '../settings';
import { TimestampHandler } from './types';
import { VideoRestrictionHandler } from '../video/restriction-handler';
import { convertTime } from './utils'; 

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
        this.videoHandler = new VideoRestrictionHandler();
    }
    
    /**
     * Apply timestamp restrictions to videos in the current view
     */
    public applyTimestampRestrictions(videosFromMarkdown: VideoWithTimestamp[]): void {
        const allVideoElementsInDom = Array.from(document.querySelectorAll('video'));
        const processedDomVideoElements = new Set<HTMLVideoElement>();

        // 1. Cleanup handlers from all video elements first
        allVideoElementsInDom.forEach(videoEl => this.videoHandler.cleanup(videoEl));

        // 2. Process videos defined in Markdown
        for (const videoData of videosFromMarkdown) {
            // Construct the expected 'src' attribute value of the parent .internal-embed div
            const expectedEmbedParentSrc = videoData.originalSubpath
                ? `${videoData.originalLinkPath}${videoData.originalSubpath}`
                : videoData.originalLinkPath;

            let matchedVideoElement: HTMLVideoElement | null = null;

            for (const videoEl of allVideoElementsInDom) {
                if (processedDomVideoElements.has(videoEl)) continue;

                const parentEmbedDiv = videoEl.closest('.internal-embed[src]');
                if (parentEmbedDiv) {
                    const actualEmbedParentSrc = (parentEmbedDiv as HTMLElement).getAttribute('src');
                    // TODO: Enhance robustness of this comparison if Obsidian URI encodes or fully resolves paths in 'src'
                    if (actualEmbedParentSrc === expectedEmbedParentSrc) {
                        matchedVideoElement = videoEl;
                        break;
                    }
                }
            }

            if (matchedVideoElement) {
                if (videoData.timestamp) {
                    // Apply timestamp from Markdown data
                    this.videoHandler.apply(
                        matchedVideoElement,
                        videoData.timestamp.start,
                        videoData.timestamp.end !== -1 ? videoData.timestamp.end : Infinity,
                        videoData.path // Use the resolved TFile path for identification
                    );
                }
                // If videoData.timestamp is null, no restrictions are applied (cleanup already handled it)
                processedDomVideoElements.add(matchedVideoElement);
            }
        }

        // 3. Process any remaining (unmanaged) video elements in the DOM
        // These might be from other plugins or direct HTML, not linked via standard Markdown
        for (const videoEl of allVideoElementsInDom) {
            if (!processedDomVideoElements.has(videoEl)) {
                const { startTime, endTime, path: domPath } = this.extractTimestampsFromDom(videoEl);
                if (startTime !== undefined) {
                    this.videoHandler.apply(
                        videoEl,
                        startTime,
                        endTime !== undefined && endTime >= 0 ? endTime : Infinity,
                        domPath || "unmanaged DOM video"
                    );
                }
                // No need to add to processedDomVideoElements here as this is the final loop for them
            }
        }
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
     * Extract timestamps from the DOM (primarily for unmanaged videos)
     * Timestamps (start/end) are ONLY taken from video.src or source child src.
     * Path can be inferred from parent if video.src is a blob.
     */
    private extractTimestampsFromDom(videoEl: HTMLVideoElement): { 
        startTime?: number; 
        endTime?: number; 
        path: string 
    } {
        let start: number | undefined;
        let end: number | undefined;
        let pathAttributeVal = ""; // Store the attribute value from which path is derived
        let foundTimestampInVideoSrc = false;

        // Priority 1: Timestamp from video.src or source tag src
        const videoSources = [videoEl.src];
        const sourceTags = videoEl.querySelectorAll('source');
        sourceTags.forEach(source => {
            if (source.src) videoSources.push(source.src);
        });

        for (const srcAttr of videoSources) {
            if (!srcAttr) continue;
            
            let currentSrcPathPart = srcAttr.split('#t=')[0];
            const srcTimeMatch = srcAttr.match(/#t=([^,]+)(?:,([^,]+))?/);

            if (srcTimeMatch && srcTimeMatch[1]) {
                // Ensure we don't use the placeholder '0.001' from media-extended
                if (srcTimeMatch[1] === '0.001' && (srcTimeMatch[2] === undefined || srcTimeMatch[2] === '0.001')) {
                    // This is likely a placeholder, ignore it for timestamping purposes
                } else {
                    const parsedStart = convertTime(srcTimeMatch[1]);
                    if (parsedStart !== null) {
                        start = parsedStart;
                        foundTimestampInVideoSrc = true; // Mark that timestamp came from video/source src
                    }
                    if (srcTimeMatch[2]) {
                        const parsedEnd = convertTime(srcTimeMatch[2]);
                        if (parsedEnd !== null) {
                            end = parsedEnd;
                        }
                    }
                    // If we found a timestamp in video.src or source.src, this src is definitive for path
                    pathAttributeVal = currentSrcPathPart;
                    break; // Found timestamp from video/source, no need to check other source tags
                }
            }
            
            // If this is the first path we've identified from video/source, store it.
            // This will be overwritten if a subsequent source tag contains a timestamp.
            if (!pathAttributeVal) { 
                pathAttributeVal = currentSrcPathPart;
            }
        }
        
        // Priority 2: Path from parent embed elements if not found or unclear from video/source src.
        // This part should NOT extract timestamps, only path information.
        if (!pathAttributeVal || pathAttributeVal.startsWith('blob:') || pathAttributeVal.startsWith('data:')) {
            const parentEl = videoEl.closest('.internal-embed.media-embed');
            if (parentEl) {
                const parentSrcAttr = (parentEl as HTMLElement).getAttribute('src');
                if (parentSrcAttr) {
                    // Only use parent's src for path, do not parse timestamp from it.
                    pathAttributeVal = parentSrcAttr.split('#t=')[0]; 
                }
            }
        }
        
        let finalPath = pathAttributeVal;
        try {
            if (pathAttributeVal && pathAttributeVal.includes('://')) {
                const url = new URL(pathAttributeVal);
                finalPath = decodeURIComponent(url.pathname); 
                if (finalPath.startsWith('/') && !/^[A-Za-z]:/.test(finalPath.substring(1))) {
                    finalPath = finalPath.substring(1);
                }
            }
        } catch (e) {
            // Not a valid URL, use pathAttributeVal as is (could be relative path)
        }

        // If a timestamp was found, it must have come from video.src or source.src
        // Otherwise, start/end remain undefined.
        return { startTime: foundTimestampInVideoSrc ? start : undefined, endTime: foundTimestampInVideoSrc ? end : undefined, path: finalPath || "" };
    }
}
