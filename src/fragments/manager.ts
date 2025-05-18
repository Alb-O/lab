import { VideoWithFragment } from '../video';
import { VideoFragmentsSettings } from '../settings';
import { FragmentHandler } from './types';
import { VideoRestrictionHandler } from '../video/restriction-handler';
import { parseFragmentToSeconds } from '../fragments/utils';

/**
 * Manages fragment restrictions for videos in Obsidian
 */
export class FragmentManager {
    private settings: VideoFragmentsSettings;
    private videoHandler: FragmentHandler;
    
    constructor(settings: VideoFragmentsSettings) {
        this.settings = settings;
        this.videoHandler = new VideoRestrictionHandler();
    }
    
    /**
     * Apply fragment restrictions to videos in the current view across specified documents
     */
    public applyFragmentRestrictions(videosFromMarkdown: VideoWithFragment[], targetDocuments: Document[]): void {
        const processedDomVideoElements = new Set<HTMLVideoElement>();

        // 1. Cleanup handlers from all video elements in all target documents first
        for (const doc of targetDocuments) {
            const allVideoElementsInDom = Array.from(doc.querySelectorAll('video'));
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
                    if (videoData.fragment) {
                        // Resolve percent-based start/end if needed
                        let resolvedStart = videoData.fragment.start;
                        let resolvedEnd = videoData.fragment.end;
                        if (this.isPercentObject(resolvedStart)) {
                            resolvedStart = matchedVideoElement.duration * (resolvedStart.percent / 100);
                        }
                        if (this.isPercentObject(resolvedEnd)) {
                            resolvedEnd = matchedVideoElement.duration * (resolvedEnd.percent / 100);
                        }
                        // Fix: always apply restrictions if fragment is present, even if resolvedStart/resolvedEnd is 0 or percent
                        this.videoHandler.apply(
                            matchedVideoElement,
                            resolvedStart,
                            (resolvedEnd !== undefined && resolvedEnd !== -1) ? resolvedEnd : Infinity,
                            videoData.path,
                            this.settings,
                            false,
                            videoData.startRaw,
                            videoData.endRaw
                        );
                    }
                    // If videoData.fragment is null, no restrictions are applied (cleanup already handled it)
                    processedDomVideoElements.add(matchedVideoElement);
                }
            }

            // 3. Process any remaining (unmanaged) video elements in the DOM
            // These might be from other plugins or direct HTML, not linked via standard Markdown
            for (const videoEl of allVideoElementsInDom) {
                if (!processedDomVideoElements.has(videoEl)) {
                    const { startTime, endTime, path: domPath } = this.extractFragmentsFromDom(videoEl);
                    let resolvedStart = startTime;
                    let resolvedEnd = endTime;
                    if (this.isPercentObject(resolvedStart)) {
                        resolvedStart = videoEl.duration * (resolvedStart.percent / 100);
                    }
                    if (this.isPercentObject(resolvedEnd)) {
                        resolvedEnd = videoEl.duration * (resolvedEnd.percent / 100);
                    }
                    if (resolvedStart !== undefined) {
                        this.videoHandler.apply(
                            videoEl,
                            resolvedStart,
                            resolvedEnd !== undefined && resolvedEnd >= 0 ? resolvedEnd : Infinity,
                            domPath || "unmanaged DOM video",
                            this.settings,
                            false,
                            undefined,
                            undefined
                        );
                    }
                    // No need to add to processedDomVideoElements here as this is the final loop for them
                }
            }
        }
    }
    
    /**
     * Clean up all fragment handlers from a video element
     */
    public cleanupHandlers(videoEl: HTMLVideoElement): void {
        this.videoHandler.cleanup(videoEl);
    }
    
    /**
     * Set up an observer for detecting new videos in a specific document
     */
    public setupVideoObserver(doc: Document, detectVideosCallback: () => void): MutationObserver {
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
        observer.observe(doc.body, { childList: true, subtree: true });
        return observer;
    }
    
    /**
     * Extract fragments from the DOM (primarily for unmanaged videos)
     * Fragments (start/end) are ONLY taken from video.src or source child src.
     * Path can be inferred from parent if video.src is a blob.
     */
    private extractFragmentsFromDom(videoEl: HTMLVideoElement): { 
        startTime?: number | { percent: number }; 
        endTime?: number | { percent: number }; 
        path: string 
    } {
        let start: number | { percent: number } | undefined;
        let end: number | { percent: number } | undefined;
        let pathAttributeVal = ""; // Store the attribute value from which path is derived
        let foundFragmentInVideoSrc = false;

        // Priority 1: Fragment from video.src or source tag src
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
                // Ensure we don't use the placeholder '0.001'
                if (srcTimeMatch[1] === '0.001' && (srcTimeMatch[2] === undefined || srcTimeMatch[2] === '0.001')) {
                } else {
                    const parsedStart = parseFragmentToSeconds(srcTimeMatch[1]);
                    if (parsedStart !== null) {
                        start = parsedStart;
                        foundFragmentInVideoSrc = true; // Mark that fragment came from video/source src
                    }
                    if (srcTimeMatch[2]) {
                        const parsedEnd = parseFragmentToSeconds(srcTimeMatch[2]);
                        if (parsedEnd !== null) {
                            end = parsedEnd;
                        }
                    }
                    // If we found a fragment in video.src or source.src, this src is definitive for path
                    pathAttributeVal = currentSrcPathPart;
                    break; // Found fragment from video/source, no need to check other source tags
                }
            }
            
            // If this is the first path we've identified from video/source, store it.
            // This will be overwritten if a subsequent source tag contains a fragment.
            if (!pathAttributeVal) { 
                pathAttributeVal = currentSrcPathPart;
            }
        }
        
        // Priority 2: Path from parent embed elements if not found or unclear from video/source src.
        // This part should NOT extract fragments, only path information.
        if (!pathAttributeVal || pathAttributeVal.startsWith('blob:') || pathAttributeVal.startsWith('data:')) {
            const parentEl = videoEl.closest('.internal-embed.media-embed');
            if (parentEl) {
                const parentSrcAttr = (parentEl as HTMLElement).getAttribute('src');
                if (parentSrcAttr) {
                    // Only use parent's src for path, do not parse fragment from it.
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

        // If a fragment was found, it must have come from video.src or source.src
        // Otherwise, start/end remain undefined.
        return { startTime: foundFragmentInVideoSrc ? start : undefined, endTime: foundFragmentInVideoSrc ? end : undefined, path: finalPath || "" };
    }

    // Helper type guard for percent object
    private isPercentObject(val: any): val is { percent: number } {
        return val && typeof val === 'object' && 'percent' in val && typeof val.percent === 'number';
    }
}
