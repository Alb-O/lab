import { VideoWithTimestamp } from './utils';
import { VideoTimestampsSettings } from './settings';

export class StatusBarController {
    private statusBarItemEl: HTMLElement;
    private settings: VideoTimestampsSettings;

    constructor(statusBarItemEl: HTMLElement, settings: VideoTimestampsSettings) {
        this.statusBarItemEl = statusBarItemEl;
        this.settings = settings;
    }

    /**
	 * Update the status bar with information about detected videos
	 */
	public updateStatusBar(videos: VideoWithTimestamp[]): void {
		if (!this.settings.showStatusBarInfo) {
			this.statusBarItemEl.setText('');
			return;
		}
		
		if (videos.length === 0) {
			this.statusBarItemEl.setText('No videos detected');
			return;
		}
		
		// Count videos with timestamps
		const videosWithTimestamps = videos.filter(v => v.timestamp !== null);
		
		if (videosWithTimestamps.length > 0) {
			this.statusBarItemEl.setText(
				`${videos.length} video${videos.length !== 1 ? 's' : ''}, ` +
				`${videosWithTimestamps.length} with timestamp${videosWithTimestamps.length !== 1 ? 's' : ''}`
			);
		} else {
			this.statusBarItemEl.setText(`${videos.length} video${videos.length !== 1 ? 's' : ''}`);
		}
	}
}
