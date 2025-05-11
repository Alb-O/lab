import { MarkdownView, Notice, Plugin, WorkspaceLeaf } from 'obsidian';
import { VideoWithTimestamp } from './utils';
import { VideoTimestampsSettings } from './settings';

export class TimestampController {
    private settings: VideoTimestampsSettings;
    private plugin: Plugin;

    constructor(settings: VideoTimestampsSettings, plugin: Plugin) {
        this.settings = settings;
        this.plugin = plugin;
    }

    public applyTimestampRestrictions(videos: VideoWithTimestamp[]): void {
        const videosWithTimestamps = videos.filter(v => v.timestamp !== null);
        if (videosWithTimestamps.length === 0) {
            return;
        }

        const videoElements = document.querySelectorAll('video');
        const videosByPath: Map<string, VideoWithTimestamp[]> = new Map();
        for (const video of videosWithTimestamps) {
            if (!videosByPath.has(video.path)) {
                videosByPath.set(video.path, []);
            }
            videosByPath.get(video.path)?.push(video);
        }

        const elementsBySource: Map<string, HTMLVideoElement[]> = new Map();
        for (const videoEl of Array.from(videoElements)) {
            const videoSrc = videoEl.src || videoEl.querySelector('source')?.src || '';
            if (!elementsBySource.has(videoSrc)) {
                elementsBySource.set(videoSrc, []);
            }
            elementsBySource.get(videoSrc)?.push(videoEl);
        }

        let isReadingMode = false;
        const activeView = this.plugin.app.workspace.getActiveViewOfType(MarkdownView);
        if (activeView && activeView.getMode() === 'preview') {
            isReadingMode = true;
        }

        const processedVideos = new Set<HTMLVideoElement>();

        // STEP 1: Direct DOM extraction
        for (const videoEl of Array.from(videoElements)) {
            if (processedVideos.has(videoEl)) continue;
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

            if (start !== undefined) {
                const startTimeSeconds = start;
                const endTimeSeconds = end !== undefined && end >= 0 ? end : Infinity;
                this.applyTimestampHandlers(videoEl, startTimeSeconds, endTimeSeconds, path || "extracted from DOM");
                processedVideos.add(videoEl);
            }
        }

        // STEP 2: Metadata matching
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
                    const endTimeSeconds = videoData.timestamp?.end !== undefined && videoData.timestamp?.end >= 0 ? videoData.timestamp?.end : Infinity;
                    this.applyTimestampHandlers(videoEl, startTimeSeconds, endTimeSeconds, videoData.path);
                    processedVideos.add(videoEl);
                }
            }
        }

        // STEP 3: Fallback for unprocessed videos
        if (processedVideos.size < videoElements.length) {
            this.handleUnprocessedVideos(videoElements, processedVideos, videosByPath);
        }
    }

    private handleUnprocessedVideos(
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
                this.applyTimestampHandlers(videoEl, startTimeSeconds, endTimeSeconds, videoData.path);
                processedVideos.add(videoEl);
            }
        }
    }

    private applyTimestampHandlers(videoEl: HTMLVideoElement, startTime: number, endTime: number, path: string): void {
        videoEl.dataset.startTime = startTime.toString();
        videoEl.dataset.endTime = endTime === Infinity ? 'end' : endTime.toString();
        videoEl.dataset.timestampPath = path;

        let isHandlingTimeUpdate = false;
        
        const setInitialTime = () => {
            if (videoEl.currentTime !== startTime) {
                 videoEl.currentTime = startTime;
            }
        };

        if (videoEl.readyState >= 1) { // HAVE_METADATA or higher
            setInitialTime();
        } else {
            videoEl.onloadedmetadata = () => {
                setInitialTime();
            };
        }

        const timeUpdateHandler = () => {
            if (isHandlingTimeUpdate) return;
            try {
                isHandlingTimeUpdate = true;
                if (videoEl.currentTime < startTime && videoEl.readyState >= 1) { // Ensure metadata loaded before seeking
                    videoEl.currentTime = startTime;
                }
                else if (endTime !== Infinity && videoEl.currentTime >= endTime) {
                    videoEl.pause();
                    if (videoEl.readyState >=1) videoEl.currentTime = endTime;
                }
            } finally {
                setTimeout(() => { isHandlingTimeUpdate = false; }, 50);
            }
        };

        if ((videoEl as any)._timestampTimeUpdateHandler) {
            videoEl.removeEventListener('timeupdate', (videoEl as any)._timestampTimeUpdateHandler);
        }
        (videoEl as any)._timestampTimeUpdateHandler = timeUpdateHandler;
        videoEl.addEventListener('timeupdate', (videoEl as any)._timestampTimeUpdateHandler);

        const seekedHandler = () => {
            if (videoEl.readyState >= 1) { // Ensure metadata loaded before seeking
                if (videoEl.currentTime < startTime) videoEl.currentTime = startTime;
                if (endTime !== Infinity && videoEl.currentTime > endTime) videoEl.currentTime = endTime;
            }
        };

        if ((videoEl as any)._timestampSeekedHandler) {
            videoEl.removeEventListener('seeked', (videoEl as any)._timestampSeekedHandler);
        }
        (videoEl as any)._timestampSeekedHandler = seekedHandler;
        videoEl.addEventListener('seeked', (videoEl as any)._timestampSeekedHandler);
    }

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
                setTimeout(() => detectVideosCallback(), 500);
            }
        });
        observer.observe(document.body, { childList: true, subtree: true });
        return observer;
    }

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