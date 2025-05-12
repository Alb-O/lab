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
        
        // Set up initial time position
        this.setInitialTime(videoEl, startTime);
        
        // Create the master handler for all events
        const masterHandler = (event: Event) => {
            const eventType = event.type;
            
            switch (eventType) {
                case 'timeupdate':
                    // Keep video within min bound during playback
                    if (videoEl.currentTime < startTime) {
                        videoEl.currentTime = startTime;
                    }
                    
                    // Handle when video reaches max time during playback
                    if (endTime !== Infinity && videoEl.currentTime >= endTime && !videoEl.paused) {
                        // Flag this as an automatic/programmatic pause
                        isProgrammaticPause = true;
                        
                        // Pause at the end time
                        videoEl.pause();
                        videoEl.currentTime = endTime;
                        
                        // Set state flags
                        state.reachedEnd = true;
                        state.autoResume = true; // Enable auto-resume for automatic pauses
                        videoEl.dataset.reachedEnd = 'true';
                        videoEl.dataset.autoResume = 'true';
                        
                        // Reset the flag after a short delay
                        setTimeout(() => { isProgrammaticPause = false; }, 20);
                    }
                    break;
                    
                case 'seeking':
                    // If we're seeking back from an automatic pause at the end, prepare to auto-play
                    if (state.autoResume && !state.userPaused) {
                        const seekingToValidPosition = 
                            videoEl.currentTime <= startTime || 
                            (videoEl.currentTime < endTime - 0.2);
                            
                        if (seekingToValidPosition) {
                            state.shouldAutoPlay = true;
                            videoEl.dataset.shouldAutoPlay = 'true';
                        }
                    }
                    break;
                    
                case 'seeked':
                    // If seeking before start, enforce minimum time
                    if (videoEl.currentTime < startTime) {
                        videoEl.currentTime = startTime;
                        
                        // Auto-play if coming back from end and not manually paused
                        if (state.shouldAutoPlay && !state.userPaused) {
                            state.shouldAutoPlay = false;
                            videoEl.dataset.shouldAutoPlay = 'false';
                            // Use direct play call for immediate response
                            videoEl.play();
                        }
                    }
                    // If seeking past end, enforce maximum and pause
                    else if (endTime !== Infinity && videoEl.currentTime > endTime) {
                        // Flag as programmatic pause
                        isProgrammaticPause = true;
                        
                        // Pause at end time
                        videoEl.pause();
                        videoEl.currentTime = endTime;
                        
                        // Set state flags
                        state.seekedPastEnd = true;
                        videoEl.dataset.seekedPastEnd = 'true';
                        
                        // Enable auto-resume unless manually paused
                        if (!state.userPaused) {
                            state.autoResume = true;
                            videoEl.dataset.autoResume = 'true';
                        }
                        
                        // Reset the flag
                        setTimeout(() => { isProgrammaticPause = false; }, 20);
                    } 
                    // If seeking to a valid position between start and end
                    else {
                        // Reset relevant flags
                        state.seekedPastEnd = false;
                        videoEl.dataset.seekedPastEnd = 'false';
                        
                        // If seeking away from the end
                        if (endTime === Infinity || videoEl.currentTime < (endTime - 0.2)) {
                            state.reachedEnd = false;
                            videoEl.dataset.reachedEnd = 'false';
                            
                            // Auto-play if conditions are met
                            if (state.shouldAutoPlay && !state.userPaused) {
                                state.shouldAutoPlay = false;
                                videoEl.dataset.shouldAutoPlay = 'false';
                                // Use direct play call
                                videoEl.play();
                            }
                        }
                    }
                    break;
                    
                case 'play':
                    // Clear user paused flag on deliberate play
                    state.userPaused = false;
                    videoEl.dataset.userPaused = 'false';
                    
                    // Prevent playback if we manually seeked past the end
                    if (state.seekedPastEnd) {
                        event.preventDefault();
                        event.stopImmediatePropagation();
                        videoEl.pause();
                        return false;
                    }
                    
                    // If we reached the end naturally, restart from beginning
                    if (state.reachedEnd && !state.seekedPastEnd) {
                        videoEl.currentTime = startTime;
                        state.reachedEnd = false;
                        videoEl.dataset.reachedEnd = 'false';
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
        videoEl.removeEventListener('pause', handler, true);
        videoEl.removeEventListener('manual-pause', handler, true);
    }
}
