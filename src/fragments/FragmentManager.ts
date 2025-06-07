import { VideoWithFragment } from '@markdown';
import { FragmentsSettings } from '@settings';
import { FragmentHandler } from '../types/types';
import { parseFragmentToSeconds } from '@utils';
import { VideoRestrictionHandler } from '@observer';

/**
 * Manages fragment restrictions for videos in Obsidian
 */
export class FragmentManager {
    private settings: FragmentsSettings;
    private videoHandler: FragmentHandler;

    constructor(settings: FragmentsSettings) {
        this.settings = settings;
        this.videoHandler = new VideoRestrictionHandler();
    }

    /**
     * Apply fragment restrictions to videos in the current view across specified documents
     */
    public applyFragmentRestrictions(videosFromMarkdown: VideoWithFragment[], targetDocuments: Document[]): void {
        const processedDomVideoElements = new Set<HTMLVideoElement>();

        // 1. First cleanup handlers from all video elements
        for (const doc of targetDocuments) {
            const allVideoElementsInDom = Array.from(doc.querySelectorAll('video'));
            allVideoElementsInDom.forEach(videoEl => this.videoHandler.cleanup(videoEl));

            // 2. Process videos defined in Markdown
            this.processMarkdownVideos(videosFromMarkdown, allVideoElementsInDom, processedDomVideoElements);

            // 3. Process any remaining unmanaged video elements in the DOM
            this.processUnmanagedVideos(allVideoElementsInDom, processedDomVideoElements);
        }
    }

    /**
     * Process videos defined in Markdown syntax
     */
    private processMarkdownVideos(
        videosFromMarkdown: VideoWithFragment[],
        allVideoElementsInDom: HTMLVideoElement[],
        processedDomVideoElements: Set<HTMLVideoElement>
    ): void {
        for (const videoData of videosFromMarkdown) {
            // Construct the expected 'src' attribute value
            const expectedEmbedParentSrc = videoData.originalSubpath
                ? `${videoData.originalLinkPath}${videoData.originalSubpath}`
                : videoData.originalLinkPath;

            const matchedVideoElement = this.findMatchingVideoElement(
                allVideoElementsInDom,
                expectedEmbedParentSrc,
                processedDomVideoElements
            );

            if (matchedVideoElement && videoData.fragment) {
                // Resolve percent-based start/end if needed
                const { start: resolvedStart, end: resolvedEnd } = this.resolvePercentValues(
                    videoData.fragment.start,
                    videoData.fragment.end,
                    matchedVideoElement.duration
                );

                this.videoHandler.apply(
                    matchedVideoElement,
                    resolvedStart,
                    (typeof resolvedEnd === 'number' && resolvedEnd !== -1) ? resolvedEnd : Infinity,
                    videoData.path,
                    this.settings,
                    false,
                    videoData.startRaw,
                    videoData.endRaw
                );

                processedDomVideoElements.add(matchedVideoElement);
            }
        }
    }

    /**
     * Process any unmanaged videos in the DOM
     */
    private processUnmanagedVideos(
        allVideoElementsInDom: HTMLVideoElement[],
        processedDomVideoElements: Set<HTMLVideoElement>
    ): void {
        for (const videoEl of allVideoElementsInDom) {
            if (!processedDomVideoElements.has(videoEl)) {
                const { startTime, endTime, path: domPath } = this.extractFragmentsFromDom(videoEl);

                if (startTime !== undefined) {
                    const { start: resolvedStart, end: resolvedEnd } = this.resolvePercentValues(
                        startTime,
                        endTime,
                        videoEl.duration
                    );

                    this.videoHandler.apply(
                        videoEl,
                        resolvedStart,
                        (typeof resolvedEnd === 'number' && resolvedEnd >= 0) ? resolvedEnd : Infinity,
                        domPath || "unmanaged DOM video",
                        this.settings,
                        false,
                        undefined,
                        undefined
                    );
                }
            }
        }
    }

    /**
     * Find a matching video element in the DOM based on embed src
     */
    private findMatchingVideoElement(
        videoElements: HTMLVideoElement[],
        expectedSrc: string,
        processedElements: Set<HTMLVideoElement>
    ): HTMLVideoElement | null {
        for (const videoEl of videoElements) {
            if (processedElements.has(videoEl)) continue;

            // Search ancestors for the embed container with a src attribute (covers div in edit mode and span in reading mode)
            let container: HTMLElement | null = videoEl.parentElement;
            while (container) {
                if (container.hasAttribute('src')) {
                    const actualSrc = container.getAttribute('src');
                    if (actualSrc === expectedSrc) {
                        return videoEl;
                    }
                    break; // found a src attribute but no match, stop traversing
                }
                container = container.parentElement;
            }
        }
        return null;
    }

    /**
     * Resolve percent-based values to absolute seconds
     */
    private resolvePercentValues(
        start: number | { percent: number } | undefined,
        end: number | { percent: number } | undefined,
        duration: number
    ): { start: number | { percent: number }, end: number | { percent: number } } {
        let resolvedStart = start;
        let resolvedEnd = end;

        if (this.isPercentObject(resolvedStart) && duration) {
            resolvedStart = duration * (resolvedStart.percent / 100);
        }

        if (this.isPercentObject(resolvedEnd) && duration) {
            resolvedEnd = duration * (resolvedEnd.percent / 100);
        }

        return { start: resolvedStart || 0, end: resolvedEnd || Infinity };
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
                setTimeout(() => detectVideosCallback(), 100);
            }
        });

        observer.observe(doc.body, { childList: true, subtree: true });
        return observer;
    }

    /**
     * Extract fragments from the DOM (primarily for unmanaged videos)
     */
    private extractFragmentsFromDom(videoEl: HTMLVideoElement): {
        startTime?: number | { percent: number };
        endTime?: number | { percent: number };
        path: string
    } {
        // First, check for fragments in parent embed container (reading mode preview)
        const parentEmbed = videoEl.closest('.internal-embed[src]');
        if (parentEmbed) {
            const parentSrcAttr = (parentEmbed as HTMLElement).getAttribute('src');
            if (parentSrcAttr) {
                const currentSrcPathPart = parentSrcAttr.split('#t=')[0];                const srcTimeMatch = parentSrcAttr.match(/#t=([^,]+)(?:,([^,]+))?/);
                if (srcTimeMatch && srcTimeMatch[1] && srcTimeMatch[1] !== '0.001') {
                    // Parse start time using enhanced parseFragmentToSeconds which now handles chrono-node parsing
                    console.log(`Extracting fragment from embed: start="${srcTimeMatch[1]}", end="${srcTimeMatch[2] || ''}"`);
                    const parsedStart = parseFragmentToSeconds(srcTimeMatch[1]);
                    console.log(`  Parsed start: ${JSON.stringify(parsedStart)}`);
                    const start = parsedStart !== null ? parsedStart : undefined;
                    // Parse end time if present
                    let end: number | { percent: number } | undefined;
                    if (srcTimeMatch[2] && srcTimeMatch[2] !== '0.001') {
                        const parsedEnd = parseFragmentToSeconds(srcTimeMatch[2]);
                        console.log(`  Parsed end: ${JSON.stringify(parsedEnd)}`);
                        if (parsedEnd !== null) end = parsedEnd;
                    }
                    return { startTime: start, endTime: end, path: currentSrcPathPart };
                }
            }
        }
        let start: number | { percent: number } | undefined;
        let end: number | { percent: number } | undefined;
        let pathAttributeVal = "";
        let foundFragmentInVideoSrc = false;

        // Get all video sources
        const videoSources = [videoEl.src];
        const sourceTags = videoEl.querySelectorAll('source');
        sourceTags.forEach(source => {
            if (source.src) videoSources.push(source.src);
        });

        // First priority: Check video.src or source tag src for fragments
        for (const srcAttr of videoSources) {
            if (!srcAttr) continue;
            const currentSrcPathPart = srcAttr.split('#t=')[0];
            const srcTimeMatch = srcAttr.match(/#t=([^,]+)(?:,([^,]+))?/);

            if (srcTimeMatch && srcTimeMatch[1] && srcTimeMatch[1] !== '0.001') {
                const parsedStart = parseFragmentToSeconds(srcTimeMatch[1]);
                if (parsedStart !== null) {
                    start = parsedStart;
                    foundFragmentInVideoSrc = true;
                }

                if (srcTimeMatch[2] && srcTimeMatch[2] !== '0.001') {
                    const parsedEnd = parseFragmentToSeconds(srcTimeMatch[2]);
                    if (parsedEnd !== null) {
                        end = parsedEnd;
                    }
                }

                // Fragment found, use this src for path
                pathAttributeVal = currentSrcPathPart;
                break;
            }

            // Store the first path we find
            if (!pathAttributeVal) {
                pathAttributeVal = currentSrcPathPart;
            }
        }

        // Second priority: Get path from parent embed if needed
        if (!pathAttributeVal || pathAttributeVal.startsWith('blob:') || pathAttributeVal.startsWith('data:')) {
            const parentEl = videoEl.closest('.internal-embed.media-embed');
            if (parentEl) {
                const parentSrcAttr = (parentEl as HTMLElement).getAttribute('src');
                if (parentSrcAttr) {
                    pathAttributeVal = parentSrcAttr.split('#t=')[0];
                }
            }
        }

        // Clean up the path
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
            // Not a valid URL, use as is
        }

        return {
            startTime: foundFragmentInVideoSrc ? start : undefined,
            endTime: foundFragmentInVideoSrc ? end : undefined,
            path: finalPath || ""
        };
    }

    // Helper type guard for percent object
    private isPercentObject(val: any): val is { percent: number } {
        return val && typeof val === 'object' && 'percent' in val && typeof val.percent === 'number';
    }
}
