import * as debug from 'debug';
import { TimestampHandler, VideoState } from '../timestamps/types';
import { updateTimelineStyles } from './styles';

/**
 * Handles video events and enforces timestamp restrictions
 */
export class VideoRestrictionHandler implements TimestampHandler {
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
            userPaused: false,
            isSeeking: false // Add a new state flag to track seeking operations
        };
        
        // Store the state object on the video element for persistence
        (videoEl as any)._timestampState = state;
        
        // Apply timeline styling if video has loaded metadata
        if (videoEl.duration) {
            updateTimelineStyles(videoEl, startTime, endTime, videoEl.duration);
        }
        
        // Add event listener for loaded metadata to style timeline when ready
        const metadataHandler = () => {
            if (videoEl.duration) {
                updateTimelineStyles(videoEl, startTime, endTime, videoEl.duration);
            }
        };
        videoEl.addEventListener('loadedmetadata', metadataHandler);
        (videoEl as any)._metadataHandler = metadataHandler;
        
        // Flag to track programmatic pauses
        let isProgrammaticPause = false;
        // Tolerance to avoid snapping when within threshold (to handle keyframe misalignment)
        const TOLERANCE = 0.05; // seconds
        // Prepare frame-based clamp callback if supported
        let frameRequestHandle: number;        
        const clampFrameCallback = (_now: number, metadata: any) => {
            // On each video frame, check if we've reached or passed the max time
            if (metadata.mediaTime >= endTime) {
                isProgrammaticPause = true;
                videoEl.pause();
                videoEl.currentTime = endTime;
                state.shouldAutoPlay = true;
                videoEl.dataset.shouldAutoPlay = 'true';
                debug.log(`[VideoTimestamps] Frame callback: Paused at endTime: ${endTime.toFixed(2)}`);
                setTimeout(() => { 
                    isProgrammaticPause = false; 
                    debug.log(`[VideoTimestamps] Frame callback: Reset isProgrammaticPause`);
                }, 50);
            } else if (metadata.mediaTime < startTime - TOLERANCE) {
                // Only clamp to minimum time if needed, but do NOT pause
                if (Math.abs(videoEl.currentTime - startTime) > TOLERANCE) {
                    videoEl.currentTime = startTime;
                    debug.log(`[VideoTimestamps] Frame callback: Clamped to startTime: ${startTime.toFixed(2)}`);
                }
            }
            
            // Schedule next frame check if video is playing
            if (!videoEl.paused) {
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
                    // Skip enforcing restrictions right after resetting from end
                    if ((videoEl as any)._justResetFromEnd) {
                        debug.log(`[VideoTimestamps] Skipping timeupdate checks - just reset from end`);
                        break;
                    }
                    
                    // Keep video within min bound during playback, with tolerance for keyframes
                    if (videoEl.currentTime < startTime - TOLERANCE) {
                        debug.log(`[VideoTimestamps] Timeupdate: currentTime (${videoEl.currentTime.toFixed(2)}) < startTime (${startTime.toFixed(2)}) - adjusting`);
                        videoEl.currentTime = startTime;
                        debug.log(`[VideoTimestamps] Set currentTime to startTime: ${startTime}`);
                    }
                    
                    // Handle when video approaches or reaches max time during playback (with tolerance)
                    if (endTime !== Infinity && videoEl.currentTime >= endTime - TOLERANCE && !videoEl.paused) {
                        debug.log(`[VideoTimestamps] Timeupdate: approaching/at endTime - currentTime: ${videoEl.currentTime.toFixed(2)}, endTime: ${endTime.toFixed(2)}`);
                        // Flag this as an automatic/programmatic pause
                        isProgrammaticPause = true;
                        debug.log(`[VideoTimestamps] Set isProgrammaticPause: true`);
                        
                        // Use VideoFrame callback if available for precise clamping
                        if ((videoEl as any).requestVideoFrameCallback) {
                            const clampFrame = (_now: number, metadata: any) => {
                                debug.log(`[VideoTimestamps] VideoFrame callback - mediaTime: ${metadata.mediaTime.toFixed(2)}, endTime: ${endTime.toFixed(2)}`);
                                // Clamp exactly at or after endTime to avoid undershoot
                                if (metadata.mediaTime >= endTime) {
                                    debug.log(`[VideoTimestamps] VideoFrame callback - mediaTime >= endTime, pausing`);
                                    videoEl.pause();
                                    videoEl.currentTime = endTime;
                                    state.shouldAutoPlay = true;
                                    videoEl.dataset.shouldAutoPlay = 'true';
                                    debug.log(`[VideoTimestamps] Set currentTime to endTime: ${endTime.toFixed(2)}`);
                                    // Reset programmatic flag after clamping
                                    setTimeout(() => { 
                                        isProgrammaticPause = false; 
                                        debug.log(`[VideoTimestamps] Reset isProgrammaticPause to false after timeout`);
                                    }, 20);
                                } else {
                                    debug.log(`[VideoTimestamps] VideoFrame callback - mediaTime < endTime, scheduling next frame check`);
                                    (videoEl as any).requestVideoFrameCallback(clampFrame);
                                }
                            };
                            debug.log(`[VideoTimestamps] Scheduling VideoFrame callback`);
                            (videoEl as any).requestVideoFrameCallback(clampFrame);
                        } else {
                            debug.log(`[VideoTimestamps] VideoFrameCallback not available, using standard pause`);
                            videoEl.pause();
                            videoEl.currentTime = endTime;
                            state.shouldAutoPlay = true;
                            videoEl.dataset.shouldAutoPlay = 'true';
                            // Enforce clamp on next frame
                            requestAnimationFrame(() => { 
                                videoEl.currentTime = endTime;
                                debug.log(`[VideoTimestamps] requestAnimationFrame - enforcing endTime: ${endTime.toFixed(2)}`); 
                            });
                            setTimeout(() => { 
                                isProgrammaticPause = false; 
                                debug.log(`[VideoTimestamps] Reset isProgrammaticPause to false after timeout`);
                            }, 20);
                        }
                        
                        // Set state flags
                        state.reachedEnd = true;
                        state.autoResume = true; // Enable auto-resume for automatic pauses
                        state.shouldAutoPlay = true; // Set shouldAutoPlay on programmatic pause
                        videoEl.dataset.reachedEnd = 'true';
                        videoEl.dataset.autoResume = 'true';
                        videoEl.dataset.shouldAutoPlay = 'true';
                        debug.log(`[VideoTimestamps] Set flags - reachedEnd: true, autoResume: true`);
                        // Reset the flag after a longer delay to ensure we don't get unwanted frames
                        setTimeout(() => { 
                            isProgrammaticPause = false;
                            debug.log(`[VideoTimestamps] Reset isProgrammaticPause to false after timeout`); 
                        }, 100);
                    }
                    break;
                case 'seeking':
                    debug.log(`[VideoTimestamps] Seeking event - currentTime: ${videoEl.currentTime.toFixed(2)}, startTime: ${startTime.toFixed(2)}, endTime: ${endTime === Infinity ? 'Infinity' : endTime.toFixed(2)}`);
                    
                    // Set seeking flag
                    state.isSeeking = true;
                    videoEl.dataset.isSeeking = 'true';
                    
                    // Track whether video was playing before seeking
                    if (!videoEl.paused) {
                        state.shouldAutoPlay = true;
                        videoEl.dataset.shouldAutoPlay = 'true';
                    }
                    
                    // Clamp seeking above max timestamp immediately to avoid overshoot
                    if (endTime !== Infinity && videoEl.currentTime > endTime + TOLERANCE) {
                        debug.log(`[VideoTimestamps] Seeking past max timestamp (${endTime.toFixed(2)})`);
                        videoEl.currentTime = endTime;
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
                        debug.log(`[VideoTimestamps] Paused video due to seeking past end time, marked as _seekedToEnd`);
                        
                        // Set a timeout to clear the seek flag after a short delay
                        // This ensures it won't affect future play button presses
                        (videoEl as any)._seekedToEndTimeout = setTimeout(() => {
                            delete (videoEl as any)._seekedToEnd;
                            delete (videoEl as any)._seekedToEndTimeout;
                            debug.log(`[VideoTimestamps] Auto-cleared _seekedToEnd flag after timeout`);
                        }, 500);
                        
                        setTimeout(() => { isProgrammaticPause = false; }, 50);
                        debug.log(`[VideoTimestamps] Set seekedPastEnd: true, userPaused: ${state.userPaused}`);
                    }
                    
                    // If seeking before start, clamp to start time
                    if (videoEl.currentTime < startTime - TOLERANCE) {
                        debug.log(`[VideoTimestamps] Seeking before startTime (${startTime.toFixed(2)})`);
                        videoEl.currentTime = startTime;
                    }
                    
                    // If we're seeking back from an automatic pause at the end, prepare to auto-play
                    if (state.autoResume && !state.userPaused) {
                        const seekingToValidPosition = 
                            videoEl.currentTime <= startTime || 
                            (videoEl.currentTime < endTime - 0.2);
                            
                        if (seekingToValidPosition) {
                            state.shouldAutoPlay = true;
                            videoEl.dataset.shouldAutoPlay = 'true';
                            debug.log(`[VideoTimestamps] Set shouldAutoPlay: true (autoResume: true, userPaused: false, seekingToValidPosition: true)`);
                        }
                    }
                    break;                
                case 'seeked':
                    debug.log(`[VideoTimestamps] Seeked event - currentTime: ${videoEl.currentTime.toFixed(2)}, startTime: ${startTime.toFixed(2)}, endTime: ${endTime === Infinity ? 'Infinity' : endTime.toFixed(2)}`);
                    debug.log(`[VideoTimestamps] State flags - reachedEnd: ${state.reachedEnd}, seekedPastEnd: ${state.seekedPastEnd}, userPaused: ${state.userPaused}, autoResume: ${state.autoResume}`);
                    
                    // Clear seeking state flag
                    state.isSeeking = false;
                    videoEl.dataset.isSeeking = 'false';
                    
                    // If ended up at exact end time, we should pause
                    if (endTime !== Infinity && Math.abs(videoEl.currentTime - endTime) < TOLERANCE) {
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
                        debug.log(`[VideoTimestamps] Auto-resuming playback after seeking`);
                        // Reset seeking flags
                        state.seekedPastEnd = false;
                        videoEl.dataset.seekedPastEnd = 'false';
                        
                        // For positions away from the end
                        if (endTime === Infinity || videoEl.currentTime < endTime - 0.2) {
                            state.reachedEnd = false;
                            videoEl.dataset.reachedEnd = 'false';
                        }
                        
                        // Reset auto-play flag and try to play
                        state.shouldAutoPlay = false;
                        videoEl.dataset.shouldAutoPlay = 'false';
                        
                        // Use setTimeout to ensure this happens after event handling is complete
                        setTimeout(() => {
                            if (!state.userPaused) {
                                videoEl.play().catch(e => debug.log(`[VideoTimestamps] Play error: ${e}`));
                            }
                        }, 0);
                    }
                    
                    // Set up frame callback for ongoing boundary enforcement if playing
                    if (!videoEl.paused && (videoEl as any).requestVideoFrameCallback) {
                        frameRequestHandle = (videoEl as any).requestVideoFrameCallback(clampFrameCallback);
                        debug.log(`[VideoTimestamps] Set up post-seek frame callback`);
                    }
                    break;                
                case 'play':
                    debug.log(`[VideoTimestamps] Play event - currentTime: ${videoEl.currentTime.toFixed(2)}, startTime: ${startTime.toFixed(2)}, endTime: ${endTime === Infinity ? 'Infinity' : endTime.toFixed(2)}`);
                    debug.log(`[VideoTimestamps] Play event flags - reachedEnd: ${state.reachedEnd}, seekedPastEnd: ${state.seekedPastEnd}, userPaused: ${state.userPaused}`);
                    
                    // User initiated play: clear userPaused
                    state.userPaused = false;
                    videoEl.dataset.userPaused = 'false';
                    
                    // Check if seekedToEnd flag is recent enough (within 300ms of seeking)
                    const isRecentSeek = (videoEl as any)._seekedToEndTimeout !== undefined;
                    
                    // Handle the end position - restart only if not directly after seeking past end
                    if (endTime !== Infinity && Math.abs(videoEl.currentTime - endTime) < TOLERANCE) {
                        // Only reset to beginning if this is a deliberate play at the end,
                        // not if we just seeked past end and are getting an automatic play event
                        if (!(videoEl as any)._seekedToEnd || !isRecentSeek) {
                            debug.log(`[VideoTimestamps] Play button pressed at end position - resetting to startTime: ${startTime}`);
                            
                            // Create a flag to prevent immediate pausing in case of event race conditions
                            (videoEl as any)._justResetFromEnd = true;
                            
                            // Reset to start time
                            videoEl.currentTime = startTime;
                            
                            // Reset state flags
                            state.reachedEnd = false;
                            videoEl.dataset.reachedEnd = 'false';
                            state.seekedPastEnd = false;
                            videoEl.dataset.seekedPastEnd = 'false';
                            
                            // Clear the prevention flag after a short delay
                            setTimeout(() => {
                                delete (videoEl as any)._justResetFromEnd;
                                debug.log(`[VideoTimestamps] Cleared reset-from-end protection flag`);
                            }, 100);
                        } else {
                            // We just seeked past end, so stay at end and don't restart
                            debug.log(`[VideoTimestamps] Not resetting to start - this was a seek past end operation`);
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
                        debug.log(`[VideoTimestamps] Set up video frame callback for end time clamping`);
                    }
                    break;
                    
                case 'pause':
                    // Critical: Only mark as user-paused if NOT a programmatic pause
                    if (!isProgrammaticPause && !state.isSeeking) {
                        state.userPaused = true;
                        videoEl.dataset.userPaused = 'true';
                        // Disable auto-resume for manual pauses
                        state.autoResume = false;
                        videoEl.dataset.autoResume = 'false';
                        debug.log(`[VideoTimestamps] User paused video`);
                    } else {
                        debug.log(`[VideoTimestamps] Programmatic pause detected, not marking as user-paused`);
                    }
                    break;
                    
                case 'manual-pause':
                    // Special event from video-controls.ts - definitely a user pause
                    state.userPaused = true;
                    videoEl.dataset.userPaused = 'true';
                    state.autoResume = false;
                    videoEl.dataset.autoResume = 'false';
                    debug.log(`[VideoTimestamps] Manual pause event received`);
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
