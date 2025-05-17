import { TimestampHandler, VideoState } from '../timestamps/types';
import { updateTimelineStyles } from './styles';
import { VideoTimestampsSettings } from '../settings';

/**
 * Handles video events and enforces timestamp restrictions
 */
export class VideoRestrictionHandler implements TimestampHandler {
    /**
     * Apply timestamp restrictions to a video element
     */
    public apply(videoEl: HTMLVideoElement, startTime: number, endTime: number, path: string, settings: VideoTimestampsSettings, skipInitialSeek = false ): void {
        this.cleanup(videoEl);

        // Store metadata on the video element
        videoEl.dataset.startTime = startTime.toString();
        videoEl.dataset.endTime = endTime === Infinity ? 'end' : endTime.toString();
        videoEl.dataset.timestampPath = path;

        // Create a state object for this video
        const state: VideoState = {
            startTime,
            endTime,
            path,
            reachedEnd: false,
            seekedPastEnd: false,
            autoResume: false,
            shouldAutoPlay: false,
            userPaused: false,
            isSeeking: false // Add a new state flag to track seeking operations
        };

        // Store the state object on the video element for persistence
        (videoEl as any)._timestampState = state;

        // Tolerance to avoid snapping when within threshold (to handle keyframe misalignment)
        const TOLERANCE = 0.001; // seconds

        // Compute a virtual end if no max timestamp
        const getEffectiveEnd = () => (endTime === Infinity && videoEl.duration > 0) ? videoEl.duration - TOLERANCE : endTime;

        // Determine if segment looping is enabled
        const doSegmentLoop = settings.loopMaxTimestamp || videoEl.loop;

        // Apply timeline styling if video has loaded metadata
        if (videoEl.duration) {
            updateTimelineStyles(videoEl, startTime, getEffectiveEnd(), videoEl.duration);
        }

        // Add event listener for loaded metadata to style timeline when ready
        const metadataHandler = () => {
            if (videoEl.duration) {
                updateTimelineStyles(videoEl, startTime, getEffectiveEnd(), videoEl.duration);
            }
        };
        videoEl.addEventListener('loadedmetadata', metadataHandler);
        (videoEl as any)._metadataHandler = metadataHandler;

        // Flag to track programmatic pauses
        let isProgrammaticPause = false;
        // Prepare frame-based clamp callback if supported
        let frameRequestHandle: number;
        const clampFrameCallback = (_now: number, metadata: any) => {
            // Skip if we've just reset from end, to allow play handler to properly restart
            if ((videoEl as any)._justResetFromEnd) {
                if (!videoEl.paused) { // Keep scheduling if meant to be playing during reset sequence
                    frameRequestHandle = (videoEl as any).requestVideoFrameCallback(clampFrameCallback);
                }
                return; // Don't apply clamping logic during the reset
            }

            // On each video frame, check if we've reached or passed the max time
            if (metadata.mediaTime >= getEffectiveEnd()) {
                if (doSegmentLoop) {
                    const wasPaused = videoEl.paused;
                    videoEl.currentTime = startTime;
                    state.reachedEnd = false; // Reset as it's looping
                    videoEl.dataset.reachedEnd = 'false';                    if (wasPaused) { // If it had paused upon reaching the end
                        videoEl.play().catch(e => {
                            if (process.env.NODE_ENV !== 'production') {
                                console.warn("Loop play failed in clampFrameCallback:", e);
                            }
                        });
                    }
                    // Continue to schedule next frame if playing
                    if (!videoEl.paused) {
                        frameRequestHandle = (videoEl as any).requestVideoFrameCallback(clampFrameCallback);
                    }
                    return; // Skip original pause logic
                } else {
                    isProgrammaticPause = true;
                    videoEl.pause();
                    videoEl.currentTime = getEffectiveEnd();
                    state.shouldAutoPlay = true;
                    videoEl.dataset.shouldAutoPlay = 'true';
                    setTimeout(() => {
                        isProgrammaticPause = false;
                    }, 50);
                }
            } else if (metadata.mediaTime < startTime - TOLERANCE) {
                // Only clamp to minimum time if needed, but do NOT pause
                if (Math.abs(videoEl.currentTime - startTime) > TOLERANCE) {
                    videoEl.currentTime = startTime;
                }
            }

            // Schedule next frame check if video is playing
            if (!videoEl.paused) {
                frameRequestHandle = (videoEl as any).requestVideoFrameCallback(clampFrameCallback);
            }
        };

        // Set up initial time position unless preserving current playback
        if (!skipInitialSeek) {
            this.setInitialTime(videoEl, startTime);
        }

        // Create the master handler for all events
        const masterHandler = (event: Event) => {
            const eventType = event.type;

            switch (eventType) {
                case 'timeupdate':
                    // Skip enforcing restrictions right after resetting from end
                    if ((videoEl as any)._justResetFromEnd) {
                        break;
                    }

                    // Keep video within min bound during playback, with tolerance for keyframes
                    if (videoEl.currentTime < startTime - TOLERANCE) {
                        videoEl.currentTime = startTime;
                    }

                    // Handle when video approaches or reaches max time
                    if (videoEl.currentTime >= getEffectiveEnd() - TOLERANCE) {
                        if (doSegmentLoop) {
                            const wasPausedAndAtEnd = videoEl.paused; // Check if it paused right at the end
                            videoEl.currentTime = startTime;
                            state.reachedEnd = false; // Looping, so not "reached end"
                            videoEl.dataset.reachedEnd = 'false';                            if (wasPausedAndAtEnd) {
                                // Attempt to resume play if it paused at the very end before looping.
                                videoEl.play().catch(e => {
                                    if (process.env.NODE_ENV !== 'production') {
                                        console.warn("Loop play failed in timeupdate:", e);
                                    }
                                });
                            }
                        } else if (!videoEl.paused) { // For non-looping, only act if playing
                            // Flag this as an automatic/programmatic pause
                            isProgrammaticPause = true;

                            // Use VideoFrame callback if available for precise clamping
                            if ((videoEl as any).requestVideoFrameCallback) {
                                const clampFrame = (_now: number, metadata: any) => {
                                    // Clamp exactly at or after endTime to avoid undershoot
                                    if (metadata.mediaTime >= getEffectiveEnd()) {
                                        videoEl.pause();
                                        videoEl.currentTime = getEffectiveEnd();
                                        state.shouldAutoPlay = true;
                                        videoEl.dataset.shouldAutoPlay = 'true';
                                        // Reset programmatic flag after clamping
                                        setTimeout(() => {
                                            isProgrammaticPause = false;
                                        }, 20);
                                    } else {
                                        (videoEl as any).requestVideoFrameCallback(clampFrame);
                                    }
                                };
                                (videoEl as any).requestVideoFrameCallback(clampFrame);
                            } else {
                                videoEl.pause();
                                videoEl.currentTime = getEffectiveEnd();
                                state.shouldAutoPlay = true;
                                videoEl.dataset.shouldAutoPlay = 'true';
                                // Enforce clamp on next frame
                                requestAnimationFrame(() => {
                                    videoEl.currentTime = getEffectiveEnd();
                                });
                                setTimeout(() => {
                                    isProgrammaticPause = false;
                                }, 20);
                            }

                            // Set state flags
                            state.reachedEnd = true;
                            state.autoResume = true; // Enable auto-resume for automatic pauses
                            state.shouldAutoPlay = true; // Set shouldAutoPlay on programmatic pause
                            videoEl.dataset.reachedEnd = 'true';
                            videoEl.dataset.autoResume = 'true';
                            videoEl.dataset.shouldAutoPlay = 'true';
                            // Reset the flag after a longer delay to ensure we don't get unwanted frames
                            setTimeout(() => {
                                isProgrammaticPause = false;
                            }, 100);
                        }
                    }
                    break;
                case 'seeking':
                    // Set seeking flag
                    state.isSeeking = true;
                    videoEl.dataset.isSeeking = 'true';

                    // Clamp dragging beyond natural end when no max timestamp
                    if (endTime === Infinity && videoEl.duration > 0 && videoEl.currentTime > videoEl.duration + TOLERANCE) {
                        videoEl.currentTime = getEffectiveEnd();
                        state.shouldAutoPlay = true;
                        videoEl.dataset.shouldAutoPlay = 'true';
                    }

                    // Track whether video was playing before seeking
                    if (!videoEl.paused) {
                        state.shouldAutoPlay = true;
                        videoEl.dataset.shouldAutoPlay = 'true';
                    }

                    // Clamp seeking above max timestamp immediately to avoid overshoot
                    if (videoEl.currentTime > getEffectiveEnd() + TOLERANCE) {
                        videoEl.currentTime = getEffectiveEnd();
                        state.seekedPastEnd = true;
                        videoEl.dataset.seekedPastEnd = 'true';
                        // Pause the video when seeking beyond end time
                        isProgrammaticPause = true;
                        videoEl.pause();

                        // Clear any existing timeout for this flag
                        if ((videoEl as any)._seekedToEndTimeout) {
                            clearTimeout((videoEl as any)._seekedToEndTimeout);
                        }

                        // Mark that this was a seek operation, not a user hitting play at end
                        (videoEl as any)._seekedToEnd = true;

                        // Set a timeout to clear the seek flag after a short delay
                        // This ensures it won't affect future play button presses
                        (videoEl as any)._seekedToEndTimeout = setTimeout(() => {
                            delete (videoEl as any)._seekedToEnd;
                            delete (videoEl as any)._seekedToEndTimeout;
                        }, 500);

                        setTimeout(() => { isProgrammaticPause = false; }, 50);
                    }

                    // If seeking before start, clamp to start time
                    if (videoEl.currentTime < startTime - TOLERANCE) {
                        videoEl.currentTime = startTime;
                    }

                    // If we're seeking back from an automatic pause at the end, prepare to auto-play
                    if (state.autoResume && !state.userPaused) {
                        const seekingToValidPosition =
                            videoEl.currentTime <= startTime ||
                            (videoEl.currentTime < getEffectiveEnd() - 0.2);

                        if (seekingToValidPosition) {
                            state.shouldAutoPlay = true;
                            videoEl.dataset.shouldAutoPlay = 'true';
                        }
                    }
                    break;
                case 'seeked':
                    // Clamp overshoot past natural end for scrubber drag
                    if (endTime === Infinity && videoEl.duration > 0 && videoEl.currentTime >= videoEl.duration - TOLERANCE) {
                        // If user dragged past the end, always clamp to just before the end
                        if (videoEl.currentTime > videoEl.duration - TOLERANCE) {
                            videoEl.currentTime = videoEl.duration - TOLERANCE;
                        }
                        state.shouldAutoPlay = true;
                        videoEl.dataset.shouldAutoPlay = 'true';
                        return;
                    }

                    // Clear seeking state flag
                    state.isSeeking = false;
                    videoEl.dataset.isSeeking = 'false';

                    // If ended up at exact end time, we should pause
                    if (Math.abs(videoEl.currentTime - getEffectiveEnd()) < TOLERANCE) {
                        if (!videoEl.paused && !state.userPaused) {
                            isProgrammaticPause = true;
                            videoEl.pause();
                            state.shouldAutoPlay = true;
                            videoEl.dataset.shouldAutoPlay = 'true';
                            setTimeout(() => { isProgrammaticPause = false; }, 50);
                        }
                    }
                    // Otherwise, auto-resume if we were playing and user didn't manually pause
                    else if (state.shouldAutoPlay && !state.userPaused) {
                        // Reset seeking flags
                        state.seekedPastEnd = false;
                        videoEl.dataset.seekedPastEnd = 'false';

                        // For positions away from the end
                        if (videoEl.currentTime < getEffectiveEnd() - 0.2) {
                            state.reachedEnd = false;
                            videoEl.dataset.reachedEnd = 'false';
                        }

                        // Reset auto-play flag and try to play
                        state.shouldAutoPlay = false;
                        videoEl.dataset.shouldAutoPlay = 'false';

                        // Use setTimeout to ensure this happens after event handling is complete
                        setTimeout(() => {
                            if (!state.userPaused) {
                                videoEl.play();
                            }
                        }, 0);
                    }

                    // Set up frame callback for ongoing boundary enforcement if playing
                    if (!videoEl.paused && (videoEl as any).requestVideoFrameCallback) {
                        frameRequestHandle = (videoEl as any).requestVideoFrameCallback(clampFrameCallback);
                    }
                    break;
                case 'play':

                    // User initiated play: clear userPaused
                    state.userPaused = false;
                    videoEl.dataset.userPaused = 'false';

                    // Check if seekedToEnd flag is recent enough (within 300ms of seeking)
                    const isRecentSeek = (videoEl as any)._seekedToEndTimeout !== undefined;

                    // Determine if we are at the effective end of the video
                    const atEffectiveEnd = Math.abs(videoEl.currentTime - getEffectiveEnd()) < TOLERANCE;

                    // Handle the end position - restart only if not directly after seeking past end
                    if (atEffectiveEnd) {
                        // Only reset to beginning if this is a deliberate play at the end,
                        // not if we just seeked past end and are getting an automatic play event
                        if (!(videoEl as any)._seekedToEnd || !isRecentSeek) {

                            // Create a flag to prevent immediate pausing in case of event race conditions
                            (videoEl as any)._justResetFromEnd = true;

                            // Reset to start time
                            if (endTime === Infinity) {
                                videoEl.currentTime = getEffectiveEnd();
                            } else {
                                videoEl.currentTime = startTime;
                            }

                            // Reset state flags
                            state.reachedEnd = false;
                            videoEl.dataset.reachedEnd = 'false';
                            state.seekedPastEnd = false;
                            videoEl.dataset.seekedPastEnd = 'false';

                            // Clear the prevention flag after a short delay
                            setTimeout(() => {
                                delete (videoEl as any)._justResetFromEnd;
                            }, 100);
                        } else {
                            // We just seeked past end, so stay at end and don't restart
                        }

                        // Always clean up the seek flag after handling to prevent interference with future play attempts
                        if ((videoEl as any)._seekedToEndTimeout) {
                            clearTimeout((videoEl as any)._seekedToEndTimeout);
                            delete (videoEl as any)._seekedToEndTimeout;
                        }
                        delete (videoEl as any)._seekedToEnd;
                    } else {
                        // Not at end position, clear seek flag if it exists
                        if ((videoEl as any)._seekedToEnd) {
                            delete (videoEl as any)._seekedToEnd;
                        }
                        if ((videoEl as any)._seekedToEndTimeout) {
                            clearTimeout((videoEl as any)._seekedToEndTimeout);
                            delete (videoEl as any)._seekedToEndTimeout;
                        }
                    }

                    // Set up frame-based clamping
                    if ((videoEl as any).requestVideoFrameCallback) {
                        frameRequestHandle = (videoEl as any).requestVideoFrameCallback(clampFrameCallback);
                    }
                    break;

                case 'pause':
                    // Handle programmatic pauses or pauses during seeking first
                    if (isProgrammaticPause || state.isSeeking) {
                        // This is not a user pause, do nothing regarding userPaused state.
                    } else if (doSegmentLoop && Math.abs(videoEl.currentTime - getEffectiveEnd()) < TOLERANCE) {
                        // If looping is enabled and video paused at the exact loop point, this is part of the loop mechanism.
                        // It should not be treated as a user pause. The loop handlers (clampFrameCallback/timeupdate)
                        // are responsible for calling play().
                        state.shouldAutoPlay = false; // Loop itself is the auto-play
                        videoEl.dataset.shouldAutoPlay = 'false';
                    } else if (endTime === Infinity && videoEl.ended && !doSegmentLoop) {
                        // Handle natural end of video ONLY IF NOT LOOPING
                        state.reachedEnd = true;
                        videoEl.dataset.reachedEnd = 'true';
                        state.autoResume = true; 
                        videoEl.dataset.autoResume = 'true';
                        state.shouldAutoPlay = true; 
                        videoEl.dataset.shouldAutoPlay = 'true';
                    } 
                    // Critical: Only mark as user-paused if not handled by above conditions.
                    else {
                        state.userPaused = true;
                        videoEl.dataset.userPaused = 'true';
                        // Disable auto-resume for manual pauses
                        state.autoResume = false;
                        videoEl.dataset.autoResume = 'false';
                    }
                    break;

                case 'manual-pause':
                    // Special event from video-controls.ts - definitely a user pause
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
        (videoEl as any)._timestampMasterHandler = masterHandler;
    }

    /**
     * Clean up all timestamp handlers from a video element
     */
    public cleanup(videoEl: HTMLVideoElement): void {
        // Remove the loadedmetadata handler if it exists
        if ((videoEl as any)._metadataHandler) {
            videoEl.removeEventListener('loadedmetadata', (videoEl as any)._metadataHandler);
            delete (videoEl as any)._metadataHandler;
        }

        const masterHandler = (videoEl as any)._timestampMasterHandler;
        if (masterHandler) {
            this.detachEventHandlers(videoEl, masterHandler);
            delete (videoEl as any)._timestampMasterHandler;
        }

        // Clean up state and data attributes
        delete (videoEl as any)._timestampState;
        delete (videoEl as any)._justResetFromEnd;
        delete (videoEl as any)._seekedToEnd;
        if ((videoEl as any)._seekedToEndTimeout) {
            clearTimeout((videoEl as any)._seekedToEndTimeout);
            delete (videoEl as any)._seekedToEndTimeout;
        }
        delete videoEl.dataset.reachedEnd;
        delete videoEl.dataset.seekedPastEnd;
        delete videoEl.dataset.autoResume;
        delete videoEl.dataset.shouldAutoPlay;
        delete videoEl.dataset.userPaused;
        delete videoEl.dataset.isSeeking;
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
 * Reapply timestamp restriction handlers without full plugin reload
 */
export function reinitializeRestrictionHandlers(settings: VideoTimestampsSettings): void {
    const handler = new VideoRestrictionHandler();
    const videos = Array.from(document.querySelectorAll('video')) as HTMLVideoElement[];
    videos.forEach(videoEl => {
        const state = (videoEl as any)._timestampState;
        if (state) {
            handler.apply(videoEl, state.startTime, state.endTime, state.path, settings, true);
        }
    });
}
