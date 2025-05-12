import { Plugin } from 'obsidian';
import { VideoTimestampsSettings } from '../settings';

/**
 * Represents the state of a video with timestamp restrictions
 */
export interface VideoState {
    startTime: number;
    endTime: number;
    path: string;
    reachedEnd: boolean;
    seekedPastEnd: boolean;
    autoResume: boolean;
    shouldAutoPlay: boolean;
    userPaused: boolean;
}

/**
 * Interface for classes that handle video timestamp enforcement
 */
export interface TimestampHandler {
    apply(videoEl: HTMLVideoElement, startTime: number, endTime: number, path: string): void;
    cleanup(videoEl: HTMLVideoElement): void;
}

/**
 * Context object for timestamp handlers
 */
export interface TimestampContext {
    settings: VideoTimestampsSettings;
    plugin: Plugin;
}
