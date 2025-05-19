import { VideoFragmentsSettings } from '@settings';

/**
 * Represents the state of a video with fragment restrictions
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
    // Preserve the user's original fragment string formats
    startRaw?: string;
    endRaw?: string;
}

/**
 * Interface for classes that handle video fragment enforcement
 */
export interface FragmentHandler {
    apply(
        videoEl: HTMLVideoElement,
        startTime: number | { percent: number },
        endTime: number | { percent: number },
        path: string,
        settings: VideoFragmentsSettings,
        skipInitialSeek?: boolean,
        startRaw?: string,
        endRaw?: string
    ): void;
    cleanup(videoEl: HTMLVideoElement): void;
}