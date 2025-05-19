import { FragmentHandler, VideoState } from '../types/types';
import { updateTimelineStyles } from '@observer';
import { VideoFragmentsSettings } from '@settings';

/**
 * Handles video events and enforces fragment restrictions
 */
export class VideoRestrictionHandler implements FragmentHandler {
    /**
     * Apply fragment restrictions to a video element
     */
    public apply(videoEl: HTMLVideoElement, startTime: number | { percent: number }, endTime: number | { percent: number }, path: string, settings: VideoFragmentsSettings, skipInitialSeek = false, startRaw?: string, endRaw?: string): void {
        this.cleanup(videoEl);

        // Helper for percent object
        function isPercentObject(val: any): val is { percent: number } {
            return val && typeof val === 'object' && 'percent' in val && typeof val.percent === 'number';
        }

        // Helper to safely get a number for restriction logic
        function safeNum(val: number | undefined, fallback: number): number {
            return typeof val === 'number' && isFinite(val) ? val : fallback;
        }

        // --- Always resolve percent values using duration, but if duration is 0, defer until loadedmetadata ---
        let resolvedStart = isPercentObject(startTime) ? (videoEl.duration && isFinite(videoEl.duration) ? videoEl.duration * (startTime.percent / 100) : undefined) : startTime;
        let resolvedEnd = isPercentObject(endTime) ? (videoEl.duration && isFinite(videoEl.duration) ? videoEl.duration * (endTime.percent / 100) : undefined) : endTime;

        // Tolerance to avoid snapping when within threshold (to handle keyframe misalignment)
        const TOLERANCE = 0.005; // seconds

        // Compute a virtual end if no max fragment
        const getEffectiveEnd = () => {
            if (resolvedEnd === undefined) return undefined;
            if (resolvedEnd === Infinity && videoEl.duration > 0 && isFinite(videoEl.duration)) {
                return videoEl.duration - TOLERANCE;
            }
            return resolvedEnd;
        };

        // Determine if segment looping is enabled
        const doSegmentLoop = settings.loopMaxFragment || videoEl.loop;

        // If percent and duration is not yet known, defer restriction logic until loadedmetadata
        if ((isPercentObject(startTime) || isPercentObject(endTime)) && (!videoEl.duration || !isFinite(videoEl.duration))) {
            const onMeta = () => {
                // Remove this handler after running
                videoEl.removeEventListener('loadedmetadata', onMeta);
                // Re-apply restrictions now that duration is known
                this.apply(
                    videoEl,
                    startTime,
                    endTime,
                    path,
                    settings,
                    skipInitialSeek,
                    startRaw,
                    endRaw
                );
            };
            videoEl.addEventListener('loadedmetadata', onMeta);
            // Set state and dataset for now, but don't enforce restrictions yet
            videoEl.dataset.fragmentPath = path;
            return;
        }

        // Store metadata on the video element
        if (startRaw != null) {
            videoEl.dataset.startTimeRaw = startRaw;
        } else {
            delete videoEl.dataset.startTimeRaw;
        }
        videoEl.dataset.startTime = (typeof resolvedStart === 'number' ? resolvedStart.toString() : '');

        if (endRaw != null) {
            videoEl.dataset.endTimeRaw = endRaw;
        } else {
            delete videoEl.dataset.endTimeRaw;
        }
        videoEl.dataset.endTime = (typeof resolvedEnd === 'number' ? (resolvedEnd === Infinity ? 'end' : resolvedEnd.toString()) : '');
        videoEl.dataset.fragmentPath = path;

        // Create a state object for this video
        const state: VideoState = {
            startTime: resolvedStart as number,
            endTime: resolvedEnd as number,
            startRaw: videoEl.dataset.startTimeRaw,
            endRaw: videoEl.dataset.endTimeRaw,
            path,
            reachedEnd: false,
            seekedPastEnd: false,
            autoResume: false,
            shouldAutoPlay: false,
            userPaused: false,
            isSeeking: false // Add a new state flag to track seeking operations
        };

        // Store the state object on the video element for persistence
        videoEl._fragmentState = state;

        // Apply timeline styling if video has loaded metadata
        if (videoEl.duration && isFinite(videoEl.duration)) {
            updateTimelineStyles(videoEl, safeNum(resolvedStart, 0), safeNum(getEffectiveEnd(), videoEl.duration), videoEl.duration);
        }

        // Add event listener for loaded metadata to style timeline when ready
        const metadataHandler = () => {
            if (videoEl.duration && isFinite(videoEl.duration)) {
                updateTimelineStyles(videoEl, safeNum(resolvedStart, 0), safeNum(getEffectiveEnd(), videoEl.duration), videoEl.duration);
            }
        };
        videoEl.addEventListener('loadedmetadata', metadataHandler);
        videoEl._metadataHandler = metadataHandler;

        // Flag to track programmatic pauses
        let isProgrammaticPause = false;
        // Prepare frame-based clamp callback if supported
        let frameRequestHandle: number;
        const clampFrameCallback = (_now: number, metadata: any) => {
            if (videoEl._justResetFromEnd) {
                if (!videoEl.paused) {
                    frameRequestHandle = videoEl.requestVideoFrameCallback(clampFrameCallback);
                }
                return;
            }
            const effEnd = getEffectiveEnd();
            if (effEnd === undefined) return;
            if (metadata.mediaTime >= effEnd) {
                if (doSegmentLoop) {
                    videoEl.currentTime = safeNum(resolvedStart, 0);
                    state.reachedEnd = false;
                    videoEl.dataset.reachedEnd = 'false';
                    if (!videoEl.paused) {
                        frameRequestHandle = videoEl.requestVideoFrameCallback(clampFrameCallback);
                    }
                    return;
                } else {
                    isProgrammaticPause = true;
                    videoEl.pause();
                    videoEl.currentTime = effEnd;
                    state.shouldAutoPlay = true;
                    videoEl.dataset.shouldAutoPlay = 'true';
                    setTimeout(() => { isProgrammaticPause = false; }, 50);
                }
            } else if (metadata.mediaTime < safeNum(resolvedStart, 0) - TOLERANCE) {
                if (Math.abs(videoEl.currentTime - safeNum(resolvedStart, 0)) > TOLERANCE) {
                    videoEl.currentTime = safeNum(resolvedStart, 0);
                }
            }
            if (!videoEl.paused) {
                frameRequestHandle = videoEl.requestVideoFrameCallback(clampFrameCallback);
            }
        };

        // Set up initial time position unless preserving current playback
        if (!skipInitialSeek) {
            this.setInitialTime(videoEl, safeNum(resolvedStart, 0));
        }

        // Create the master handler for all events
        const masterHandler = (event: Event) => {
            const eventType = event.type;
            const effEnd = getEffectiveEnd();
            switch (eventType) {
                case 'timeupdate':
                    if (videoEl._justResetFromEnd) break;
                    if (videoEl.currentTime < safeNum(resolvedStart, 0) - TOLERANCE) {
                        videoEl.currentTime = safeNum(resolvedStart, 0);
                    }
                    if (effEnd !== undefined && videoEl.currentTime >= effEnd - TOLERANCE) {
                        if (doSegmentLoop) {
                            videoEl.currentTime = safeNum(resolvedStart, 0);
                            state.reachedEnd = false;
                            videoEl.dataset.reachedEnd = 'false';
                        } else if (!videoEl.paused) {
                            isProgrammaticPause = true;
                            if (videoEl.requestVideoFrameCallback) {
                                const clampFrame = (_now: number, metadata: any) => {
                                    if (effEnd !== undefined && metadata.mediaTime >= effEnd) {
                                        videoEl.pause();
                                        videoEl.currentTime = effEnd;
                                        state.shouldAutoPlay = true;
                                        videoEl.dataset.shouldAutoPlay = 'true';
                                        setTimeout(() => { isProgrammaticPause = false; }, 20);
                                    } else {
                                        videoEl.requestVideoFrameCallback(clampFrame);
                                    }
                                };
                                videoEl.requestVideoFrameCallback(clampFrame);
                            } else {
                                videoEl.pause();
                                videoEl.currentTime = effEnd;
                                state.shouldAutoPlay = true;
                                videoEl.dataset.shouldAutoPlay = 'true';
                                requestAnimationFrame(() => {
                                    videoEl.currentTime = effEnd;
                                });
                                setTimeout(() => { isProgrammaticPause = false; }, 20);
                            }
                            state.reachedEnd = true;
                            state.autoResume = true;
                            state.shouldAutoPlay = true;
                            videoEl.dataset.reachedEnd = 'true';
                            videoEl.dataset.autoResume = 'true';
                            videoEl.dataset.shouldAutoPlay = 'true';
                            setTimeout(() => { isProgrammaticPause = false; }, 100);
                        }
                    }
                    break;
                case 'seeking':
                    state.isSeeking = true;
                    videoEl.dataset.isSeeking = 'true';
                    if (effEnd !== undefined && resolvedEnd === Infinity && videoEl.duration > 0 && videoEl.currentTime > videoEl.duration + TOLERANCE) {
                        videoEl.currentTime = effEnd;
                        state.shouldAutoPlay = true;
                        videoEl.dataset.shouldAutoPlay = 'true';
                    }
                    if (!videoEl.paused) {
                        state.shouldAutoPlay = true;
                        videoEl.dataset.shouldAutoPlay = 'true';
                    }
                    if (effEnd !== undefined && videoEl.currentTime > effEnd + TOLERANCE) {
                        videoEl.currentTime = effEnd;
                        state.seekedPastEnd = true;
                        videoEl.dataset.seekedPastEnd = 'true';
                        isProgrammaticPause = true;
                        videoEl.pause();
                        if (videoEl._seekedToEndTimeout) {
                            clearTimeout(videoEl._seekedToEndTimeout);
                        }
                        videoEl._seekedToEnd = true;
                        videoEl._seekedToEndTimeout = setTimeout(() => {
                            delete videoEl._seekedToEnd;
                            delete videoEl._seekedToEndTimeout;
                        }, 500);
                        setTimeout(() => { isProgrammaticPause = false; }, 50);
                    }
                    if (videoEl.currentTime < safeNum(resolvedStart, 0) - TOLERANCE) {
                        videoEl.currentTime = safeNum(resolvedStart, 0);
                    }
                    if (state.autoResume && !state.userPaused) {
                        const seekingToValidPosition =
                            videoEl.currentTime <= safeNum(resolvedStart, 0) ||
                            (effEnd !== undefined && videoEl.currentTime < effEnd - 0.2);
                        if (seekingToValidPosition) {
                            state.shouldAutoPlay = true;
                            videoEl.dataset.shouldAutoPlay = 'true';
                        }
                    }
                    break;
                case 'seeked':
                    if (effEnd !== undefined && resolvedEnd === Infinity && videoEl.duration > 0 && videoEl.currentTime >= videoEl.duration - TOLERANCE) {
                        if (videoEl.currentTime > videoEl.duration - TOLERANCE) {
                            videoEl.currentTime = videoEl.duration - TOLERANCE;
                        }
                        state.shouldAutoPlay = true;
                        videoEl.dataset.shouldAutoPlay = 'true';
                        return;
                    }
                    state.isSeeking = false;
                    videoEl.dataset.isSeeking = 'false';
                    if (effEnd !== undefined && Math.abs(videoEl.currentTime - effEnd) < TOLERANCE) {
                        if (!videoEl.paused && !state.userPaused) {
                            isProgrammaticPause = true;
                            videoEl.pause();
                            state.shouldAutoPlay = true;
                            videoEl.dataset.shouldAutoPlay = 'true';
                            setTimeout(() => { isProgrammaticPause = false; }, 50);
                        }
                    } else if (state.shouldAutoPlay && !state.userPaused) {
                        state.seekedPastEnd = false;
                        videoEl.dataset.seekedPastEnd = 'false';
                        if (effEnd !== undefined && videoEl.currentTime < effEnd - 0.2) {
                            state.reachedEnd = false;
                            videoEl.dataset.reachedEnd = 'false';
                        }
                        state.shouldAutoPlay = false;
                        videoEl.dataset.shouldAutoPlay = 'false';
                        setTimeout(() => {
                            if (!state.userPaused) {
                                videoEl.play();
                            }
                        }, 0);
                    }
                    if (!videoEl.paused && videoEl.requestVideoFrameCallback) {
                        frameRequestHandle = videoEl.requestVideoFrameCallback(clampFrameCallback);
                    }
                    break;
                case 'play':
                    state.userPaused = false;
                    videoEl.dataset.userPaused = 'false';
                    const isRecentSeek = videoEl._seekedToEndTimeout !== undefined;
                    const atEffectiveEnd = effEnd !== undefined && Math.abs(videoEl.currentTime - effEnd) < TOLERANCE;
                    if (atEffectiveEnd) {
                        if (!videoEl._seekedToEnd || !isRecentSeek) {
                            videoEl._justResetFromEnd = true;
                            if (resolvedEnd === Infinity && effEnd !== undefined) {
                                videoEl.currentTime = effEnd;
                            } else {
                                videoEl.currentTime = safeNum(resolvedStart, 0);
                            }
                            state.reachedEnd = false;
                            videoEl.dataset.reachedEnd = 'false';
                            state.seekedPastEnd = false;
                            videoEl.dataset.seekedPastEnd = 'false';
                            setTimeout(() => {
                                delete videoEl._justResetFromEnd;
                            }, 100);
                        }
                        if (videoEl._seekedToEndTimeout) {
                            clearTimeout(videoEl._seekedToEndTimeout);
                            delete videoEl._seekedToEndTimeout;
                        }
                        delete videoEl._seekedToEnd;
                    } else {
                        if (videoEl._seekedToEnd) {
                            delete videoEl._seekedToEnd;
                        }
                        if (videoEl._seekedToEndTimeout) {
                            clearTimeout(videoEl._seekedToEndTimeout);
                        }
                    }
                    if (videoEl.requestVideoFrameCallback) {
                        frameRequestHandle = videoEl.requestVideoFrameCallback(clampFrameCallback);
                    }
                    break;
                case 'pause':
                    if (isProgrammaticPause || state.isSeeking) {
                        // Do nothing
                    } else if (doSegmentLoop && effEnd !== undefined && Math.abs(videoEl.currentTime - effEnd) < TOLERANCE) {
                        state.shouldAutoPlay = false;
                        videoEl.dataset.shouldAutoPlay = 'false';
                    } else if (resolvedEnd === Infinity && videoEl.ended && !doSegmentLoop) {
                        state.reachedEnd = true;
                        videoEl.dataset.reachedEnd = 'true';
                        state.autoResume = true;
                        videoEl.dataset.autoResume = 'true';
                        state.shouldAutoPlay = true;
                        videoEl.dataset.shouldAutoPlay = 'true';
                    } else {
                        state.userPaused = true;
                        videoEl.dataset.userPaused = 'true';
                        state.autoResume = false;
                        videoEl.dataset.autoResume = 'false';
                    }
                    break;
                case 'manual-pause':
                    state.userPaused = true;
                    videoEl.dataset.userPaused = 'true';
                    state.autoResume = false;
                    videoEl.dataset.autoResume = 'false';
                    break;
            }
        };

        // Initialize the data attributes with defaults
        videoEl.dataset.reachedEnd = 'false';
        videoEl.dataset.seekedPastEnd = 'false';
        videoEl.dataset.autoResume = 'false';
        videoEl.dataset.shouldAutoPlay = 'false';
        videoEl.dataset.userPaused = 'false';
        videoEl.dataset.isSeeking = 'false';

        // Add all event listeners with capture phase to ensure they run first
        this.attachEventHandlers(videoEl, masterHandler);

        // Store the handler reference for cleanup
        videoEl._fragmentMasterHandler = masterHandler;
    }

    /**
     * Clean up all fragment handlers from a video element
     */
    public cleanup(videoEl: HTMLVideoElement): void {
        // Remove the loadedmetadata handler if it exists
        if (videoEl._metadataHandler) {
            videoEl.removeEventListener('loadedmetadata', videoEl._metadataHandler);
            delete videoEl._metadataHandler;
        }

        const masterHandler = videoEl._fragmentMasterHandler;
        if (masterHandler) {
            this.detachEventHandlers(videoEl, masterHandler);
            delete videoEl._fragmentMasterHandler;
        }

        // Clean up state and data attributes
        delete videoEl._fragmentState;
        delete videoEl._justResetFromEnd;
        delete videoEl._seekedToEnd;
        if (videoEl._seekedToEndTimeout) {
            clearTimeout(videoEl._seekedToEndTimeout);
            delete videoEl._seekedToEndTimeout;
        }
        delete videoEl.dataset.reachedEnd;
        delete videoEl.dataset.seekedPastEnd;
        delete videoEl.dataset.autoResume;
        delete videoEl.dataset.shouldAutoPlay;
        delete videoEl.dataset.userPaused;
        delete videoEl.dataset.isSeeking;
        delete videoEl.dataset.startTimeRaw;
        delete videoEl.dataset.endTimeRaw;
    }

    /**
     * Set the initial time position for a video
     */
    private setInitialTime(videoEl: HTMLVideoElement, startTime: number): void {
        if (videoEl.readyState >= 1) {
            // Only set if needed to avoid unnecessary seeking
            if (Math.abs(videoEl.currentTime - startTime) > 0.1) {
                videoEl.currentTime = startTime;
            }
        } else {
            videoEl.addEventListener('loadedmetadata', () => {
                videoEl.currentTime = startTime;
            }, { once: true });
        }
    }

    /**
     * Attach event handlers to a video element
     */
    private attachEventHandlers(videoEl: HTMLVideoElement, handler: (event: Event) => void): void {
        // Use capture phase to ensure our handlers run before default handlers
        videoEl.addEventListener('timeupdate', handler, true);
        videoEl.addEventListener('seeking', handler, true);
        videoEl.addEventListener('seeked', handler, true);
        videoEl.addEventListener('play', handler, true);
        videoEl.addEventListener('playing', handler, true);
        videoEl.addEventListener('pause', handler, true);
        videoEl.addEventListener('manual-pause', handler, true);
    }

    /**
     * Detach event handlers from a video element
     */
    private detachEventHandlers(videoEl: HTMLVideoElement, handler: (event: Event) => void): void {
        videoEl.removeEventListener('timeupdate', handler, true);
        videoEl.removeEventListener('seeking', handler, true);
        videoEl.removeEventListener('seeked', handler, true);
        videoEl.removeEventListener('play', handler, true);
        videoEl.removeEventListener('playing', handler, true);
        videoEl.removeEventListener('pause', handler, true);
        videoEl.removeEventListener('manual-pause', handler, true);
    }
}

/**
 * Reapply fragment restriction handlers without full plugin reload
 */
export function reinitializeRestrictionHandlers(settings: VideoFragmentsSettings): void {
    const handler = new VideoRestrictionHandler();
    const videos = Array.from(document.querySelectorAll('video')) as HTMLVideoElement[];
    videos.forEach(videoEl => {
        const state = videoEl._fragmentState;
        if (state) {
            handler.apply(videoEl, state.startTime, state.endTime, state.path, settings, true, state.startRaw, state.endRaw);
        }
    });
}
