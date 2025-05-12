export { TimestampManager } from './timestamp-manager';
export { VideoEventHandler } from './video-event-handler';
export * from './types';

// Re-export main functionality for direct use
import { TimestampManager } from './timestamp-manager';
export { TimestampManager as TimestampController };
