import { TimestampHandler, VideoState } from './types';

/**
 * Handles video events and enforces timestamp restrictions
 */
export class VideoEventHandler implements TimestampHandler {
    /**
     * Apply timestamp restrictions to a video element
     */
    public apply(videoEl: HTMLVideoElement, startTime: number, endTime: number, path: string): void {
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
            userPaused: false
        };
        
        // Store the state object on the video element for persistence
        (videoEl as any)._timestampState = state;
        
        // Flag to track programmatic pauses
        let isProgrammaticPause = false;
        // Tolerance to avoid snapping when within threshold (to handle keyframe misalignment)
        const TOLERANCE = 0.05; // seconds
        // Prepare frame-based clamp callback if supported
        let frameRequestHandle: number;        const clampFrameCallback = (_now: number, metadata: any) => {
            // On each video frame, check if we've reached or passed the max time
            if (metadata.mediaTime >= endTime) {
                isProgrammaticPause = true;
                videoEl.pause();
                videoEl.currentTime = endTime;
                console.log(`[VideoTimestamps] Frame callback: Paused at endTime: ${endTime.toFixed(2)}`);
                // flag reset
                setTimeout(() => { 
                    isProgrammaticPause = false; 
                    console.log(`[VideoTimestamps] Frame callback: Reset isProgrammaticPause`);
                }, 50);
            } else if (metadata.mediaTime < startTime - TOLERANCE) {
                // Also clamp to minimum time if needed
                isProgrammaticPause = true;
                videoEl.currentTime = startTime;
                console.log(`[VideoTimestamps] Frame callback: Clamped to startTime: ${startTime.toFixed(2)}`);
                // flag reset
                setTimeout(() => { 
                    isProgrammaticPause = false; 
                    console.log(`[VideoTimestamps] Frame callback: Reset isProgrammaticPause`);
                }, 50);
            } else {
                // Schedule next frame check
                frameRequestHandle = (videoEl as any).requestVideoFrameCallback(clampFrameCallback);
            }
        };
        
        // Set up initial time position
        this.setInitialTime(videoEl, startTime);
        
        // Create the master handler for all events
        const masterHandler = (event: Event) => {
            const eventType = event.type;
            
            switch (eventType) {
                case 'timeupdate':
                    // Keep video within min bound during playback, with tolerance for keyframes
                    if (videoEl.currentTime < startTime - TOLERANCE) {
                        console.log(`[VideoTimestamps] Timeupdate: currentTime (${videoEl.currentTime.toFixed(2)}) < startTime (${startTime.toFixed(2)}) - adjusting`);
                        videoEl.currentTime = startTime;
                        console.log(`[VideoTimestamps] Set currentTime to startTime: ${startTime}`);
                    }
                    
                    // Handle when video approaches or reaches max time during playback (with tolerance)
                    if (endTime !== Infinity && videoEl.currentTime >= endTime - TOLERANCE && !videoEl.paused) {
                        console.log(`[VideoTimestamps] Timeupdate: approaching/at endTime - currentTime: ${videoEl.currentTime.toFixed(2)}, endTime: ${endTime.toFixed(2)}`);
                        // Flag this as an automatic/programmatic pause
                        isProgrammaticPause = true;
                        console.log(`[VideoTimestamps] Set isProgrammaticPause: true`);
                        
                        // Use VideoFrame callback if available for precise clamping
                        if ((videoEl as any).requestVideoFrameCallback) {
                            const clampFrame = (_now: number, metadata: any) => {
                                console.log(`[VideoTimestamps] VideoFrame callback - mediaTime: ${metadata.mediaTime.toFixed(2)}, endTime: ${endTime.toFixed(2)}`);
                                // Clamp exactly at or after endTime to avoid undershoot
                                if (metadata.mediaTime >= endTime) {
                                    console.log(`[VideoTimestamps] VideoFrame callback - mediaTime >= endTime, pausing`);
                                    videoEl.pause();
                                    videoEl.currentTime = endTime;
                                    console.log(`[VideoTimestamps] Set currentTime to endTime: ${endTime.toFixed(2)}`);
                                    // Reset programmatic flag after clamping
                                    setTimeout(() => { 
                                        isProgrammaticPause = false; 
                                        console.log(`[VideoTimestamps] Reset isProgrammaticPause to false after timeout`);
                                    }, 20);
                                } else {
                                    console.log(`[VideoTimestamps] VideoFrame callback - mediaTime < endTime, scheduling next frame check`);
                                    (videoEl as any).requestVideoFrameCallback(clampFrame);
                                }
                            };
                            console.log(`[VideoTimestamps] Scheduling VideoFrame callback`);
                            (videoEl as any).requestVideoFrameCallback(clampFrame);
                        } else {
                            console.log(`[VideoTimestamps] VideoFrameCallback not available, using standard pause`);
                            videoEl.pause();
                            videoEl.currentTime = endTime;
                            // Enforce clamp on next frame
                            requestAnimationFrame(() => { 
                                videoEl.currentTime = endTime;
                                console.log(`[VideoTimestamps] requestAnimationFrame - enforcing endTime: ${endTime.toFixed(2)}`); 
                            });
                            setTimeout(() => { 
                                isProgrammaticPause = false; 
                                console.log(`[VideoTimestamps] Reset isProgrammaticPause to false after timeout`);
                            }, 20);
                        }
                        
                        // Set state flags
                        state.reachedEnd = true;
                        state.autoResume = true; // Enable auto-resume for automatic pauses
                        videoEl.dataset.reachedEnd = 'true';
                        videoEl.dataset.autoResume = 'true';
                        console.log(`[VideoTimestamps] Set flags - reachedEnd: true, autoResume: true`);
                          // Reset the flag after a longer delay to ensure we don't get unwanted frames
                        setTimeout(() => { 
                            isProgrammaticPause = false;
                            console.log(`[VideoTimestamps] Reset isProgrammaticPause to false after timeout`); 
                        }, 100);
                    }
                    break;
                      case 'seeking':
                    console.log(`[VideoTimestamps] Seeking event - currentTime: ${videoEl.currentTime.toFixed(2)}, startTime: ${startTime.toFixed(2)}, endTime: ${endTime === Infinity ? 'Infinity' : endTime.toFixed(2)}`);
                    // Immediately pause video during any seeking operation to prevent unwanted playback
                    if (!videoEl.paused) {
                        isProgrammaticPause = true;
                        videoEl.pause();
                        console.log(`[VideoTimestamps] Paused video during seeking operation`);
                    }
                    
                    // Clamp seeking above max timestamp immediately to avoid overshoot
                    if (endTime !== Infinity && videoEl.currentTime > endTime + TOLERANCE) {
                        console.log(`[VideoTimestamps] Seeking past max timestamp (${endTime.toFixed(2)}), setting isProgrammaticPause: true`);
                        isProgrammaticPause = true;
                        videoEl.currentTime = endTime;
                        state.seekedPastEnd = true;
                        videoEl.dataset.seekedPastEnd = 'true';
                        console.log(`[VideoTimestamps] Set seekedPastEnd: true, userPaused: ${state.userPaused}`);
                        // Reset flag shortly after
                        setTimeout(() => { 
                            isProgrammaticPause = false; 
                            console.log(`[VideoTimestamps] Reset isProgrammaticPause to false after timeout`);
                        }, 50); // Increased timeout for more reliability
                        break;
                    }
                    // If we're seeking back from an automatic pause at the end, prepare to auto-play
                    if (state.autoResume && !state.userPaused) {
                        const seekingToValidPosition = 
                            videoEl.currentTime <= startTime || 
                            (videoEl.currentTime < endTime - 0.2);
                            
                        if (seekingToValidPosition) {
                            state.shouldAutoPlay = true;
                            videoEl.dataset.shouldAutoPlay = 'true';
                            console.log(`[VideoTimestamps] Set shouldAutoPlay: true (autoResume: true, userPaused: false, seekingToValidPosition: true)`);
                        }
                    }
                    break;                case 'seeked':
                    console.log(`[VideoTimestamps] Seeked event - currentTime: ${videoEl.currentTime.toFixed(2)}, startTime: ${startTime.toFixed(2)}, endTime: ${endTime === Infinity ? 'Infinity' : endTime.toFixed(2)}`);
                    console.log(`[VideoTimestamps] State flags - reachedEnd: ${state.reachedEnd}, seekedPastEnd: ${state.seekedPastEnd}, userPaused: ${state.userPaused}, autoResume: ${state.autoResume}`);
                    
                    // If seeking before start (beyond tolerance), enforce minimum time and preserve play state
                    if (videoEl.currentTime < startTime - TOLERANCE) {
                        const wasPaused = videoEl.paused;
                        console.log(`[VideoTimestamps] Seeking before start - adjusting to startTime: ${startTime}. Was paused: ${wasPaused}`);
                        videoEl.currentTime = startTime;
                        
                        // Reset seekedPastEnd flag if it was set
                        if (state.seekedPastEnd) {
                            state.seekedPastEnd = false;
                            videoEl.dataset.seekedPastEnd = 'false';
                            console.log(`[VideoTimestamps] Reset seekedPastEnd flag when seeking to start`);
                        }
                        
                        if (!wasPaused) {
                            const playPromise = videoEl.play();
                            if (playPromise) {
                                playPromise.catch(() => {});
                            }
                        } else {
                            videoEl.pause();
                        }
                        break;
                    }
                    // If seeking past end, enforce maximum and pause
                    else if (endTime !== Infinity && videoEl.currentTime > endTime) {
                        console.log(`[VideoTimestamps] Seeking past end - currentTime: ${videoEl.currentTime.toFixed(2)} > endTime: ${endTime.toFixed(2)}`);
                        // Flag as programmatic pause
                        isProgrammaticPause = true;
                        console.log(`[VideoTimestamps] Setting isProgrammaticPause: ${isProgrammaticPause}`);
                        
                        // Pause at end time
                        videoEl.pause();
                        videoEl.currentTime = endTime;
                        console.log(`[VideoTimestamps] Paused and set currentTime to endTime: ${endTime.toFixed(2)}, now at: ${videoEl.currentTime.toFixed(2)}`);
                        
                        // Set state flags
                        state.seekedPastEnd = true;
                        videoEl.dataset.seekedPastEnd = 'true';
                        console.log(`[VideoTimestamps] Set seekedPastEnd: true`);
                        
                        // Enable auto-resume unless manually paused
                        if (!state.userPaused) {
                            state.autoResume = true;
                            videoEl.dataset.autoResume = 'true';
                            console.log(`[VideoTimestamps] Set autoResume: true (user was not paused)`);
                        } else {
                            console.log(`[VideoTimestamps] User had manually paused, not setting autoResume`);
                        }
                        
                        // Reset the flag
                        setTimeout(() => { 
                            isProgrammaticPause = false; 
                            console.log(`[VideoTimestamps] Reset isProgrammaticPause to false after timeout`);
                        }, 20);
                    }                    // If seeking to a valid position between start and end, but not exactly at end (to handle 
                    // seeked events that follow a "seeking past end" that was corrected)
                    else if (endTime === Infinity || videoEl.currentTime < endTime - 0.01) {
                        // Reset relevant flags
                        state.seekedPastEnd = false;
                        videoEl.dataset.seekedPastEnd = 'false';
                        console.log(`[VideoTimestamps] Valid seek position - seekedPastEnd: false`);
                        
                        // If seeking away from the end
                        if (endTime === Infinity || videoEl.currentTime < (endTime - 0.2)) {
                            state.reachedEnd = false;
                            videoEl.dataset.reachedEnd = 'false';
                            console.log(`[VideoTimestamps] Seeking away from end - reachedEnd: false`);
                            
                            // Auto-play if conditions are met
                            if (state.shouldAutoPlay && !state.userPaused) {
                                state.shouldAutoPlay = false;
                                videoEl.dataset.shouldAutoPlay = 'false';
                                console.log(`[VideoTimestamps] Auto-playing because shouldAutoPlay: true and userPaused: false`);
                                // Use direct play call and swallow promise errors
                                const playPromise = videoEl.play();
                                if (playPromise) {
                                    playPromise.catch(() => {});
                                }
                            } else {
                                console.log(`[VideoTimestamps] Not auto-playing - shouldAutoPlay: ${state.shouldAutoPlay}, userPaused: ${state.userPaused}`);
                            }
                        } else {
                            console.log(`[VideoTimestamps] Near end but within bounds, not resetting reachedEnd flag`);
                        }                    } else {
                        console.log(`[VideoTimestamps] At exact endTime (${endTime.toFixed(2)}), preserving seekedPastEnd flag: ${state.seekedPastEnd}`);
                        // Only handle automatic reset in the play event handler, not in the seeked event handler
                        // This prevents auto-reset when just seeking to the end without playing
                    }
                    break;                case 'play':
                    console.log(`[VideoTimestamps] Play event - currentTime: ${videoEl.currentTime.toFixed(2)}, startTime: ${startTime.toFixed(2)}, endTime: ${endTime === Infinity ? 'Infinity' : endTime.toFixed(2)}`);
                    console.log(`[VideoTimestamps] Play event flags - reachedEnd: ${state.reachedEnd}, seekedPastEnd: ${state.seekedPastEnd}, userPaused: ${state.userPaused}`);
                    
                    // For play attempts after seeking past end, just reset the flags but keep current position
                    if (state.seekedPastEnd && endTime !== Infinity && Math.abs(videoEl.currentTime - endTime) < TOLERANCE) {
                        console.log(`[VideoTimestamps] User explicitly played after seekedPastEnd - just resetting flags without repositioning`);
                        state.seekedPastEnd = false;
                        videoEl.dataset.seekedPastEnd = 'false';
                        state.reachedEnd = false;
                        // No seeking here, allow playback from current position
                        break; // allow play to continue
                    }
                    
                    // Prevent playback if we manually seeked past the end (and not at exact endTime)
                    if (state.seekedPastEnd) {
                        console.log(`[VideoTimestamps] Preventing play because seekedPastEnd: true`);
                        event.preventDefault();
                        event.stopImmediatePropagation();
                        videoEl.pause();
                        return false;
                    }
                      // If user hits play at the end timestamp after natural playback completion, restart from beginning
                    if (endTime !== Infinity && videoEl.currentTime >= endTime - TOLERANCE && !state.seekedPastEnd) {
                        console.log(`[VideoTimestamps] Playing at/after endTime after natural completion - resetting to startTime: ${startTime}`);
                        state.reachedEnd = false;
                        videoEl.currentTime = startTime;
                        break; // allow play to continue
                    }
                    
                    // Clear user paused flag on deliberate play
                    state.userPaused = false;
                    videoEl.dataset.userPaused = 'false';
                    
                    // If we reached the end naturally, restart from beginning
                    if (state.reachedEnd && !state.seekedPastEnd) {
                        console.log(`[VideoTimestamps] Reached end naturally, restarting from startTime: ${startTime}`);
                        videoEl.currentTime = startTime;
                        state.reachedEnd = false;
                        videoEl.dataset.reachedEnd = 'false';
                    }

                    // Schedule frame-based clamping when playing
                    if ((videoEl as any).requestVideoFrameCallback) {
                        frameRequestHandle = (videoEl as any).requestVideoFrameCallback(clampFrameCallback);
                        console.log(`[VideoTimestamps] Set up video frame callback for end time clamping`);
                    }
                    break;
                    
                case 'pause':
                    // Critical: Only mark as user-paused if NOT a programmatic pause
                    if (!isProgrammaticPause) {
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
        
        // Add all event listeners with capture phase to ensure they run first
        this.attachEventHandlers(videoEl, masterHandler);
        
        // Store the handler reference for cleanup
        (videoEl as any)._timestampMasterHandler = masterHandler;
    }
    
    /**
     * Clean up all timestamp handlers from a video element
     */
    public cleanup(videoEl: HTMLVideoElement): void {
        const masterHandler = (videoEl as any)._timestampMasterHandler;
        if (masterHandler) {
            this.detachEventHandlers(videoEl, masterHandler);
            delete (videoEl as any)._timestampMasterHandler;
        }
        
        // Clean up state and data attributes
        delete (videoEl as any)._timestampState;
        delete videoEl.dataset.reachedEnd;
        delete videoEl.dataset.seekedPastEnd;
        delete videoEl.dataset.autoResume;
        delete videoEl.dataset.shouldAutoPlay;
        delete videoEl.dataset.userPaused;
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
