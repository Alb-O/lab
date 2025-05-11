import { MarkdownView, Notice, Plugin, WorkspaceLeaf } from 'obsidian';
import { VideoDetector } from './video-detector';
import { DEFAULT_SETTINGS, IVideoTimestampsPlugin, VideoTimestampsSettings, VideoTimestampsSettingTab } from './settings';
import { VideoWithTimestamp } from './utils';

export default class VideoTimestamps extends Plugin implements IVideoTimestampsPlugin {
	settings: VideoTimestampsSettings;
	videoDetector: VideoDetector;
	statusBarItemEl: HTMLElement;
	async onload() {
		// Load settings and initialize the video detector
		this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());
		this.videoDetector = new VideoDetector();
		
		// Create a status bar item to show detected videos
		this.statusBarItemEl = this.addStatusBarItem();
		this.statusBarItemEl.setText('No videos detected');
		this.statusBarItemEl.addClass('video-timestamps-status');
		
		// Add a ribbon icon to manually trigger video detection
		const ribbonIconEl = this.addRibbonIcon('video', 'Detect Videos', (evt: MouseEvent) => {
			// Manually trigger video detection when clicked
			this.detectVideosInActiveView();
			new Notice('Video detection complete');
		});
		ribbonIconEl.addClass('video-timestamps-ribbon');

		// Register for file changes to update video detection
		this.registerEvent(
			this.app.workspace.on('active-leaf-change', (leaf) => {
				if (leaf) {
					this.handleActiveLeafChange(leaf);
				}
			})
		);
		
		// Register for file content changes
		this.registerEvent(
			this.app.metadataCache.on('changed', (file) => {
				const activeView = this.app.workspace.getActiveViewOfType(MarkdownView);
				if (activeView && activeView.file && activeView.file.path === file.path) {
					// Clear the cache and reprocess when file content changes
					this.videoDetector.clearCache();
					this.detectVideosInActiveView();
				}
			})
		);
		
		// Add a command to detect videos in current view
		this.addCommand({
			id: 'detect-videos-in-current-view',
			name: 'Detect videos in current view',
			callback: () => {
				const videos = this.detectVideosInActiveView();
				new Notice(`Detected ${videos.length} video${videos.length !== 1 ? 's' : ''}`);
			}
		});
		
		// Set up MutationObserver to watch for dynamically added videos
		this.setupVideoObserver();
		
		// Add a settings tab
		this.addSettingTab(new VideoTimestampsSettingTab(this.app, this));
		
		// Initial detection on load
		this.detectVideosInActiveView();
	}
	
	onunload() {
		// Clean up any resources or event listeners
		console.log('Video Timestamps plugin unloaded');
	}
	
	/**
	 * Handle when the active leaf changes in Obsidian
	 */
	private handleActiveLeafChange(leaf: WorkspaceLeaf): void {
		if (leaf.view instanceof MarkdownView) {
			// Only process markdown views
			this.detectVideosInActiveView();
		} else {
			// Clear status bar when not in a markdown view
			this.updateStatusBar([]);
		}
	}
	/**
	 * Detect videos in all open markdown views
	 * @returns Array of detected videos with timestamps across all views
	 */
	public detectVideosInActiveView(): VideoWithTimestamp[] {
		console.log('Debug - detectVideosInActiveView called');
		
		// Get all leaf views that contain markdown
		const markdownViews: MarkdownView[] = [];
		this.app.workspace.iterateAllLeaves(leaf => {
			if (leaf.view instanceof MarkdownView) {
				markdownViews.push(leaf.view);
			}
		});
		
		console.log(`Debug - Found ${markdownViews.length} markdown views`);
		
		if (markdownViews.length === 0) {
			console.log('Debug - No markdown views found');
			return [];
		}
		
		// Collect videos from all markdown views
		const allVideos: VideoWithTimestamp[] = [];
		for (const view of markdownViews) {
			const videos = this.videoDetector.getVideosFromActiveView(view);
			console.log(`Debug - Detected ${videos.length} videos in view: ${view.file?.path}`);
			allVideos.push(...videos);
		}
		
		console.log('Debug - Total videos detected across all views:', allVideos.length, allVideos);
		
		// Update the status bar with videos from the active view only
		const activeView = this.app.workspace.getActiveViewOfType(MarkdownView);
		if (activeView) {
			const activeViewVideos = allVideos.filter(v => v.file?.path === activeView.file?.path);
			this.updateStatusBar(activeViewVideos);
		} else {
			this.updateStatusBar([]);
		}
		
		// Apply timestamp restrictions to all video elements
		this.applyTimestampRestrictions(allVideos);
		
		// Show debug info if enabled
		if (this.settings.debugMode && allVideos.length > 0) {
			this.videoDetector.debugVideos(allVideos);
		}
		
		return allVideos;
	}
	
	/**
	 * Update the status bar with information about detected videos
	 */
	private updateStatusBar(videos: VideoWithTimestamp[]): void {
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
	}	/**
	 * Apply timestamp restrictions to video elements in the document
	 * @param videos Array of detected videos with timestamps
	 */
	private applyTimestampRestrictions(videos: VideoWithTimestamp[]): void {
		// Only process videos with timestamps
		const videosWithTimestamps = videos.filter(v => v.timestamp !== null);
		
		console.log('Debug - videosWithTimestamps:', videosWithTimestamps);
		
		if (videosWithTimestamps.length === 0) {
			console.log('Debug - No videos with timestamps found');
			return;
		}
		
		// Find all video elements in the document
		const videoElements = document.querySelectorAll('video');
		console.log('Debug - Found video elements:', videoElements.length);
		
		// Debug all video sources
		videoElements.forEach((el, i) => {
			const src = el.src || el.querySelector('source')?.src || '';
			console.log(`Debug - Video element ${i} source:`, src);
		});
		
		// Group all videos by file path 
		// This allows us to identify multiple instances of the same file with different timestamps
		const videosByPath: Map<string, VideoWithTimestamp[]> = new Map();
		
		for (const video of videosWithTimestamps) {
			if (!videosByPath.has(video.path)) {
				videosByPath.set(video.path, []);
			}
			videosByPath.get(video.path)?.push(video);
		}
				// Debug the video groups
		for (const [path, videoGroup] of videosByPath.entries()) {
			console.log(`Debug - File ${path} has ${videoGroup.length} instances with timestamps:`);
			videoGroup.forEach((v, i) => {
				console.log(`  ${i}: ${v.linktext} (${v.timestamp?.start}s to ${v.timestamp?.end !== undefined && v.timestamp?.end >= 0 ? v.timestamp?.end + 's' : 'end'})`);
			});
		}
		
		// Group video elements by their source
		const elementsBySource: Map<string, HTMLVideoElement[]> = new Map();
		
		for (const videoEl of Array.from(videoElements)) {
			const videoSrc = videoEl.src || videoEl.querySelector('source')?.src || '';
			if (!elementsBySource.has(videoSrc)) {
				elementsBySource.set(videoSrc, []);
			}
			elementsBySource.get(videoSrc)?.push(videoEl);
		}
		
		// Debug the element groups
		for (const [src, elements] of elementsBySource.entries()) {
			console.log(`Debug - Source ${src} has ${elements.length} elements`);
		}
		
		// For each path, match video elements to the video metadata based on position
		for (const [path, videoGroup] of videosByPath.entries()) {
			// Find all elements that contain this path in their source
			const matchingElements: HTMLVideoElement[] = [];
			
			for (const [src, elements] of elementsBySource.entries()) {
				if (src.includes(path)) {
					matchingElements.push(...elements);
				}
			}
			
			console.log(`Debug - Path ${path} matched ${matchingElements.length} elements, and has ${videoGroup.length} metadata objects`);
			
			// Match elements to metadata objects based on their position
			// We assume elements appear in the DOM in the same order they're defined in markdown
			const maxToProcess = Math.min(matchingElements.length, videoGroup.length);
			
			for (let i = 0; i < maxToProcess; i++) {
				const videoEl = matchingElements[i];
				const videoData = videoGroup[i];
				console.log(`Debug - Applying restrictions to video ${i}:`, videoData.path);
				console.log('Debug - Full link:', videoData.linktext);
				
				if (videoData.timestamp) {
					// Get start time in seconds
					const startTimeSeconds = videoData.timestamp.start;
					// Get end time in seconds (if specified)
					const endTimeSeconds = videoData.timestamp?.end !== undefined && videoData.timestamp?.end >= 0 ? videoData.timestamp?.end : Infinity;
					
					console.log('Debug - Setting timestamp range:', startTimeSeconds, 'to', endTimeSeconds);
						
					// Set initial playback position to the start timestamp
					videoEl.currentTime = startTimeSeconds;
					
					// Add event listener to restrict playback to the timestamp range
					const restrictSeekHandler = () => {
						console.log(`Debug - Current time: ${videoEl.currentTime}, Range: ${startTimeSeconds} to ${endTimeSeconds}`);
						
						// If before start time, move to start
						if (videoEl.currentTime < startTimeSeconds) {
							console.log('Debug - Correcting playback position to start time');
							videoEl.currentTime = startTimeSeconds;
						}
						// If after end time (and end time is specified), move to start
						else if (endTimeSeconds !== Infinity && videoEl.currentTime > endTimeSeconds) {
							console.log('Debug - Reached end time, restarting from start time');
							videoEl.currentTime = startTimeSeconds;
						}
					};
					
					// Remove any existing handler first to avoid duplicates
					videoEl.removeEventListener('timeupdate', restrictSeekHandler);
					videoEl.addEventListener('timeupdate', restrictSeekHandler);
					
					// Also handle seeking directly
					videoEl.removeEventListener('seeked', restrictSeekHandler);
					videoEl.addEventListener('seeked', restrictSeekHandler);
						
					// Handle reaching the end of the range
					const timeRangeEndHandler = () => {
						if (endTimeSeconds !== Infinity) {
							// If we're at or past the end time, pause the video and set time exactly to end
							if (videoEl.currentTime >= endTimeSeconds) {
								console.log('Debug - Reached end time, pausing video');
								videoEl.pause();
								videoEl.currentTime = endTimeSeconds; // Ensure we stop exactly at end point
							}
						}
					};
					videoEl.removeEventListener('timeupdate', timeRangeEndHandler);
					videoEl.addEventListener('timeupdate', timeRangeEndHandler);
					
					console.log(`Applied timestamp range: ${startTimeSeconds}s to ${endTimeSeconds === Infinity ? 'end' : endTimeSeconds + 's'} for video ${videoData.path}`);
				}
			}
		}
	}
	
	/**
	 * Set up a MutationObserver to watch for dynamically added video elements
	 */
	private setupVideoObserver(): void {
		// Create a MutationObserver to watch for new video elements
		const observer = new MutationObserver((mutations) => {
			let videoAdded = false;
			
			// Check if any videos were added
			for (const mutation of mutations) {
				if (mutation.type === 'childList') {
					for (const node of Array.from(mutation.addedNodes)) {
						// Check if the added node is a video or contains videos
						if (node instanceof HTMLVideoElement) {
							videoAdded = true;
							break;
						}
						
						if (node instanceof Element) {
							const videos = node.querySelectorAll('video');
							if (videos.length > 0) {
								videoAdded = true;
								break;
							}
						}
					}
				}
			}
			
			// If videos were added, re-apply timestamp restrictions
			if (videoAdded) {
				console.log('Debug - New video element detected, reapplying timestamp restrictions');
				setTimeout(() => this.detectVideosInActiveView(), 500); // Small delay to ensure video is fully loaded
			}
		});
		
		// Start observing the document with the configured parameters
		observer.observe(document.body, { childList: true, subtree: true });
		
		// Store the observer for cleanup
		this.register(() => observer.disconnect());
	}
	
	async saveSettings() {
		await this.saveData(this.settings);
	}
}
