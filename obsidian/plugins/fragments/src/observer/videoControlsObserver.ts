import { observeElements } from './dom-observer';

/**
 * Sets up hover behavior and play/pause state classes for video elements
 */
export function setupVideoControls(getAllRelevantDocuments: () => Document[]): void {
	observeElements(getAllRelevantDocuments, 'video', setupSingleVideoControls);
}

/**
 * Attach event handlers to a single video element
 */
function setupSingleVideoControls(videoEl: HTMLVideoElement): void {
	// Check if we've already set up this video
	if (videoEl.dataset.controlsInitialized === 'true') {
		return;
	}

	// Handle initial state
	if (videoEl.paused) {
		videoEl.classList.add('paused');
	}

	// Create event handlers
	const pauseHandler = () => {
		videoEl.classList.add('paused');
		videoEl.dispatchEvent(new Event('mouseleave')); // Hide controls when paused
	};

	const playHandler = () => {
		videoEl.classList.remove('paused');
	};

	// Track clicks on the video and pause buttons specifically
	const clickHandler = (e: MouseEvent) => {
		// If the video is being paused by the user clicking the pause button or video itself
		if (!videoEl.paused) {
			const isPauseButton = (e.target as HTMLElement)?.closest('.video-controls-pause, .video-pause-button');

			if (isPauseButton || e.target === videoEl) {
				// Dispatch a special event to indicate this is a manual pause
				setTimeout(() => {
					if (videoEl.paused) {
						videoEl.dispatchEvent(new Event('manual-pause'));
					}
				}, 50);
			}
		}
	};

	// Play/pause toggle
	videoEl.addEventListener('pause', pauseHandler);
	videoEl.addEventListener('play', playHandler);
	videoEl.addEventListener('click', clickHandler);

	// If we have controls, monitor them specifically
	const pauseButton = videoEl.closest('.video-container')?.querySelector('.video-controls-pause, .video-pause-button');
	if (pauseButton) {
		pauseButton.addEventListener('click', clickHandler);
	}

	// Hover toggles
	videoEl.addEventListener('mouseenter', () => videoEl.classList.add('hovered'));
	videoEl.addEventListener('mouseleave', () => videoEl.classList.remove('hovered'));

	// Mark as initialized
	videoEl.dataset.controlsInitialized = 'true';
}