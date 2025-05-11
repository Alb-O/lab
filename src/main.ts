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
				// Determine if we're in reading or preview mode by analyzing the DOM structure
		// The logic is more complex than just checking for a class
		let isReadingMode = false;
		let isSourceMode = false;
		
		// First check if any video elements have parent with media-embed class
		for (const el of Array.from(videoElements)) {
			const parentEl = el.closest('.internal-embed.media-embed');
			if (parentEl) {
				// Check if the parent has src attribute which indicates reading mode
				const parentSpan = parentEl as HTMLElement;
				const srcAttr = parentSpan.getAttribute('src');
				const altAttr = parentSpan.getAttribute('alt');
				
				// In true reading mode, the span would have src and possibly alt attributes
				// Debug what we found
				console.log(`Debug - Video parent attributes:`, { src: srcAttr, alt: altAttr });
				
				if (srcAttr && (srcAttr.includes('#t=') || (altAttr && altAttr.includes(' > t=')))) {
					isReadingMode = true;
					console.log('Debug - Identified as reading mode based on timestamp attributes');
					break;
				}
				
				// Check if we're in source/live preview mode
				const contentContainer = parentEl.closest('[data-mode="source"]');
				if (contentContainer) {
					isSourceMode = true;
					console.log('Debug - Identified as source mode based on data-mode attribute');
					break;
				}
			}
		}
		
		console.log('Debug - Mode detection:', { isReadingMode, isSourceMode });
		
		// In Reading mode, let's also debug the DOM structure around video elements to understand what we're dealing with
		if (isReadingMode) {
			videoElements.forEach((el, i) => {
				const parent = el.parentElement;
				const grandparent = parent?.parentElement;
				const greatGrandparent = grandparent?.parentElement;
				
				console.log(`Debug - Video element ${i} DOM hierarchy:`);
				console.log(`  - Video classes: ${el.className}`);
				console.log(`  - Parent tag: ${parent?.tagName}, classes: ${parent?.className}`);
				console.log(`  - Grandparent tag: ${grandparent?.tagName}, classes: ${grandparent?.className}`);
				console.log(`  - Great-grandparent tag: ${greatGrandparent?.tagName}, classes: ${greatGrandparent?.className}`);
				
				// Try to find any data attributes that might contain timestamp info
				const allParents = [];
				let currentEl: Element | null = el;
				while (currentEl) {
					allParents.push(currentEl);
					currentEl = currentEl.parentElement;
				}
				
				// Check for data attributes in any parent element
				for (const parentEl of allParents) {
					const dataAttrs = Array.from(parentEl.attributes)
						.filter(attr => attr.name.startsWith('data-'))
						.map(attr => `${attr.name}="${attr.value}"`);
					
					if (dataAttrs.length > 0) {
						console.log(`  - Found data attributes in ${parentEl.tagName}:`, dataAttrs.join(', '));
					}
				}
			});
		}
		
		// Track which videos have been processed
		const processedVideos = new Set<HTMLVideoElement>();
		
		// STEP 1: Try to extract timestamps directly from video elements or their containers
		// This approach works for both reading mode and live preview when the timestamp info is in the DOM
		for (const videoEl of Array.from(videoElements)) {
			// Skip if already processed
			if (processedVideos.has(videoEl)) continue;
			
			let start: number | undefined;
			let end: number | undefined;
			let path = "";
					// First try to get timing info from video source URL
			const videoSrc = videoEl.src || videoEl.querySelector('source')?.src || '';
			const srcTimeMatch = videoSrc.match(/#t=([0-9:.]+),?([0-9:.]+)?/);
			
			// Skip default value of 0.001 which Obsidian adds for playback position preservation
			if (srcTimeMatch && srcTimeMatch[1] !== '0.001') {
				console.log('Debug - Found timestamp in video source URL:', srcTimeMatch);
				start = this.parseTimeToSeconds(srcTimeMatch[1]);
				end = srcTimeMatch[2] ? this.parseTimeToSeconds(srcTimeMatch[2]) : undefined;
			}
					// If no timing in source, try parent element (for reading mode)
			if (!start) {
				const parentEl = videoEl.closest('.internal-embed.media-embed');
				if (parentEl) {
					const parentElem = parentEl as HTMLElement;
					const altText = parentElem.getAttribute('alt');
					const srcText = parentElem.getAttribute('src');
					
					console.log('Debug - Checking parent element attributes:', { alt: altText, src: srcText });
					
					// Try alt attribute first (format: "filename > t=start,end")
					if (altText && altText.includes(' > t=')) {
						const timeMatch = altText.match(/ > t=([0-9:.]+),?([0-9:.]+)?/);
						if (timeMatch) {
							console.log('Debug - Found timestamp in alt attribute:', timeMatch);
							start = this.parseTimeToSeconds(timeMatch[1]);
							end = timeMatch[2] ? this.parseTimeToSeconds(timeMatch[2]) : undefined;
							
							// Get file path from alt text
							const pathMatch = altText.match(/^(.+?) >/);
							if (pathMatch) {
								path = pathMatch[1];
							}
						}
					}
					
					// Then try src attribute (format: "filename#t=start,end")
					if (!start && srcText) {
						const timeMatch = srcText.match(/#t=([0-9:.]+),?([0-9:.]+)?/);
						if (timeMatch) {
							console.log('Debug - Found timestamp in src attribute:', timeMatch);
							start = this.parseTimeToSeconds(timeMatch[1]);
							end = timeMatch[2] ? this.parseTimeToSeconds(timeMatch[2]) : undefined;
							
							// Get file path from src text
							const pathMatch = srcText.match(/^(.+?)#t=/);
							if (pathMatch) {
								path = pathMatch[1];
							}
						}
					}
				}
			}
			
			// Apply timestamp restriction if we found start time
			if (start !== undefined) {
				const startTimeSeconds = start;
				const endTimeSeconds = end !== undefined && end >= 0 ? end : Infinity;
				
				console.log(`Debug - Direct DOM approach: Setting range ${startTimeSeconds}s to ${endTimeSeconds === Infinity ? 'end' : endTimeSeconds + 's'}`);
				
				// Apply timestamp handlers
				this.applyTimestampHandlers(videoEl, startTimeSeconds, endTimeSeconds, path || "extracted from DOM");
				
				// Mark as processed
				processedVideos.add(videoEl);
			}
		}
		
		// STEP 2: For videos still not processed, try matching them with metadata
		// This works best in Live Preview mode or when we have good metadata
		
		// For each path, match video elements to the video metadata based on position
		for (const [path, videoGroup] of videosByPath.entries()) {
			// Skip empty groups
			if (videoGroup.length === 0) continue;
			
			// Find all elements that match this path which haven't been processed yet
			const matchingElements: HTMLVideoElement[] = [];
			
			for (const [src, elements] of elementsBySource.entries()) {
				// In Reading mode, match more flexibly by filename
				if (isReadingMode) {
					// Extract just the filename from the path to match against src
					const filename = path.split('/').pop()?.split('\\').pop();
					if (filename) {
						// Add only unprocessed elements
						elements.forEach(el => {
							if (!processedVideos.has(el) && src.includes(filename)) {
								matchingElements.push(el);
							}
						});
					}
				} else {
					// In Live Preview, match more strictly by path
					// Add only unprocessed elements
					elements.forEach(el => {
						if (!processedVideos.has(el) && src.includes(path)) {
							matchingElements.push(el);
						}
					});
				}
			}
			
			console.log(`Debug - Path ${path} matched ${matchingElements.length} elements, and has ${videoGroup.length} metadata objects`);
			
			// Match elements to metadata objects based on their position
			// We assume videos appear in the DOM in the same order they're defined in markdown
			const maxToProcess = Math.min(matchingElements.length, videoGroup.length);
			
			for (let i = 0; i < maxToProcess; i++) {
				const videoEl = matchingElements[i];
				const videoData = videoGroup[i];
				
				console.log(`Debug - Metadata approach: Applying restrictions to video ${i}:`, videoData.path);
				console.log('Debug - Full link:', videoData.linktext);
				
				if (videoData.timestamp) {
					// Get start time in seconds
					const startTimeSeconds = videoData.timestamp.start;
					// Get end time in seconds (if specified)
					const endTimeSeconds = videoData.timestamp?.end !== undefined && videoData.timestamp?.end >= 0 ? videoData.timestamp?.end : Infinity;
					
					console.log('Debug - Setting timestamp range:', startTimeSeconds, 'to', endTimeSeconds);
					
					// Apply timestamp handlers
					this.applyTimestampHandlers(videoEl, startTimeSeconds, endTimeSeconds, videoData.path);
					
					// Mark as processed
					processedVideos.add(videoEl);
				}
			}
		}
		
		// STEP 3: For any remaining unprocessed videos, use the fallback method
		// This handles rare cases where we couldn't match using the above methods
		if (processedVideos.size < videoElements.length) {
			console.log(`Debug - ${videoElements.length - processedVideos.size} videos remain unprocessed, trying fallback method`);
			this.handleUnprocessedVideos(videoElements, processedVideos, videosByPath);
		}
	}
	
	/**
	 * Handle videos that weren't processed by direct DOM extraction
	 */
	private handleUnprocessedVideos(
		allVideos: NodeListOf<HTMLVideoElement>, 
		processedVideos: Set<HTMLVideoElement>,
		videosByPath: Map<string, VideoWithTimestamp[]>
	): void {
		// Create a copy of unprocessed video elements
		const unprocessedVideos = Array.from(allVideos).filter(v => !processedVideos.has(v));
		
		if (unprocessedVideos.length === 0) {
			return;
		}
		
		console.log(`Debug - Processing ${unprocessedVideos.length} unprocessed videos using metadata`);
		
		// Collect all video metadata into a flat array
		const allVideoData: VideoWithTimestamp[] = [];
		for (const group of videosByPath.values()) {
			allVideoData.push(...group);
		}
		
		// Skip if no metadata to match
		if (allVideoData.length === 0) {
			console.log('Debug - No video metadata available for matching');
			return;
		}
		
		console.log(`Debug - Have ${allVideoData.length} video metadata to match`);
		
		// Assign metadata to videos based on their index order
		// This is a fallback assuming videos are in the same order as metadata
		const maxToProcess = Math.min(unprocessedVideos.length, allVideoData.length);
		
		for (let i = 0; i < maxToProcess; i++) {
			const videoEl = unprocessedVideos[i];
			const videoData = allVideoData[i];
			
			if (!videoData.timestamp) {
				continue;
			}
			
			console.log(`Debug - Fallback: Applying metadata from ${videoData.linktext} to video ${i}`);
			
			const startTimeSeconds = videoData.timestamp.start;
			const endTimeSeconds = videoData.timestamp?.end !== undefined && videoData.timestamp?.end >= 0 
				? videoData.timestamp?.end 
				: Infinity;
			
			this.applyTimestampHandlers(videoEl, startTimeSeconds, endTimeSeconds, videoData.path);
		}
	}
	
	/**
	 * Apply timestamp handlers to a video element
	 */	private applyTimestampHandlers(videoEl: HTMLVideoElement, startTime: number, endTime: number, path: string): void {
		// First, add a custom data attribute to mark this video with its timestamp range
		// This helps with debugging and ensuring we're applying the right restrictions
		videoEl.dataset.startTime = startTime.toString();
		videoEl.dataset.endTime = endTime === Infinity ? 'end' : endTime.toString();
		videoEl.dataset.timestampPath = path;
		
		// Set initial playback position to the start timestamp
		videoEl.currentTime = startTime;
		
		// Add event listener to restrict playback to the timestamp range
		const restrictSeekHandler = () => {
			// Only log every 1s to reduce console spam
			if (Math.floor(videoEl.currentTime * 10) % 10 === 0) {
				console.log(`Debug - Video ${path}: Current time: ${videoEl.currentTime}, Range: ${startTime} to ${endTime === Infinity ? 'end' : endTime}`);
			}
			
			// If before start time, move to start
			if (videoEl.currentTime < startTime) {
				console.log(`Debug - Video ${path}: Correcting playback position to start time ${startTime}`);
				videoEl.currentTime = startTime;
			}
			// If after end time (and end time is specified), move to start
			else if (endTime !== Infinity && videoEl.currentTime > endTime) {
				console.log(`Debug - Video ${path}: Reached end time ${endTime}, restarting from start time ${startTime}`);
				videoEl.currentTime = startTime;
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
			if (endTime !== Infinity) {
				// If we're at or past the end time, pause the video and set time exactly to end
				if (videoEl.currentTime >= endTime) {
					console.log(`Debug - Video ${path}: Reached end time ${endTime}, pausing video`);
					videoEl.pause();
					videoEl.currentTime = endTime; // Ensure we stop exactly at end point
				}
			}
		};
		videoEl.removeEventListener('timeupdate', timeRangeEndHandler);
		videoEl.addEventListener('timeupdate', timeRangeEndHandler);
		
		console.log(`Applied timestamp range: ${startTime}s to ${endTime === Infinity ? 'end' : endTime + 's'} for video ${path}`);
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
	
	/**
	 * Parse a time string (like 1:30 or 90) to seconds
	 * @param timeStr Time string in format "minutes:seconds" or just seconds
	 * @returns Time in seconds
	 */
	private parseTimeToSeconds(timeStr: string): number {
		// If it's just a number, return it directly
		if (!timeStr.includes(':')) {
			return parseFloat(timeStr);
		}
		
		// Otherwise, parse as minutes:seconds
		const parts = timeStr.split(':');
		const minutes = parseInt(parts[0], 10);
		const seconds = parseFloat(parts[1]);
		
		return minutes * 60 + seconds;
	}
	
	async saveSettings() {
		await this.saveData(this.settings);
	}
}
