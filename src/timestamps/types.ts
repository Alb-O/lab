import { VideoTimestampsSettings } from '../settings';

/**
 * Represents the state of a video with timestamp restrictions
 */
export interface VideoState {
    startTime: number | { percent: number };
    endTime: number | { percent: number };
    path: string;
    reachedEnd: boolean;
    seekedPastEnd: boolean;
    autoResume: boolean;
    shouldAutoPlay: boolean;
    userPaused: boolean;
    isSeeking: boolean;
    // Preserve the user's original timestamp string formats
    startRaw?: string;
    endRaw?: string;
}

/**
 * Interface for classes that handle video timestamp enforcement
 */
export interface TimestampHandler {
    apply(
        videoEl: HTMLVideoElement,
        startTime: number | { percent: number },
        endTime: number | { percent: number },
        path: string,
        settings: VideoTimestampsSettings,
        skipInitialSeek?: boolean,
        startRaw?: string,
        endRaw?: string
    ): void;
    cleanup(videoEl: HTMLVideoElement): void;
}