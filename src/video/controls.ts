/**
 * Sets up hover behavior and play/pause state classes for video elements
 */
export function setupVideoControls(): void {
  // Observe for video elements being added to the DOM
  const observer = new MutationObserver((mutations) => {
    let videoAdded = false;
    for (const mutation of mutations) {
      if (mutation.type === 'childList') {
        for (const node of Array.from(mutation.addedNodes)) {
          if (node instanceof HTMLVideoElement || (node instanceof Element && node.querySelector('video'))) {
            videoAdded = true;
            break;
          }
        }
      }
      if (videoAdded) {
        setupAllVideoControls();
        break;
      }
    }
  });
  
  observer.observe(document.body, { childList: true, subtree: true });
  
  // Initial setup for existing videos
  setupAllVideoControls();
}

/**
 * Attach event handlers to all video elements in the document
 */
function setupAllVideoControls(): void {
  document.querySelectorAll('video').forEach(setupSingleVideoControls);
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
  
  // Add filename label inline before the video container
  const fileName = videoEl.dataset.timestampPath
      || videoEl.src.split('/').pop() 
      || '';
  const label = document.createElement('div');
  label.className = 'video-timestamps-filename';
  label.textContent = fileName;
  const container = videoEl.parentElement;
  // insert label as a sibling before the container
  if (container?.parentElement) {
      container.parentElement.insertBefore(label, container);
  }

  // Mark as initialized
  videoEl.dataset.controlsInitialized = 'true';
}

/**
 * Clears custom timeline styling for the allowed segment.
 */
export function clearTimelineStyles(videoEl: HTMLVideoElement): void {
  // Remove debug overlay if it exists
  const overlay = (videoEl as any)._debugOverlay as HTMLElement;
  if (overlay) {
    overlay.remove();
    delete (videoEl as any)._debugOverlay;
  }
}

/**
 * Updates the video timeline to visually represent the allowed segment.
 */
export function updateTimelineStyles(
  videoEl: HTMLVideoElement,
  startTime: number,
  endTime: number,
  duration: number
): void {
  clearTimelineStyles(videoEl);
  if (!duration || !isFinite(duration) || duration <= 0) return;

  const startPercent = Math.max(0, Math.min(100, (startTime / duration) * 100));
  const endPercent = endTime === Infinity
    ? 100
    : Math.max(0, Math.min(100, (endTime / duration) * 100));

  addVisualTimelineDebugOverlay(videoEl, startPercent, endPercent);
}

/**
 * Add a visual overlay for allowed/restricted regions.
 */
function addVisualTimelineDebugOverlay(
  videoEl: HTMLVideoElement,
  startPercent: number,
  endPercent: number
): void {
  const container = videoEl.parentElement;
  if (!container) return;
  container.style.position = container.style.position || 'relative';

  const overlay = document.createElement('div');
  overlay.className = 'video-timestamps-debug-overlay';

  // build segment definitions
  const segments = [
    { cls: 'video-timestamps-before-segment', width: startPercent },
    { cls: 'video-timestamps-allowed-segment', width: endPercent - startPercent },
    { cls: 'video-timestamps-after-segment', width: 100 - endPercent }
  ];

  // only append segments with positive width
  for (const seg of segments) {
    if (seg.width > 0) {
      const el = document.createElement('div');
      el.className = seg.cls;
      el.style.width = `${seg.width}%`;
      overlay.appendChild(el);
    }
  }

  container.appendChild(overlay);
  (videoEl as any)._debugOverlay = overlay;
}

/**
 * Dump basic video structure
 */
function dumpVideoStructure(videoEl: HTMLVideoElement): void {
  console.debug('[VideoTimestamps] Video element:', videoEl);
  console.debug('[VideoTimestamps] Video controls enabled:', videoEl.controls);
  console.debug('[VideoTimestamps] Video classes:', videoEl.className);
  console.debug('[VideoTimestamps] Video parent:', videoEl.parentElement);
}

// Helper function kept for backwards compatibility
function getTimelineSegments(videoEl: HTMLVideoElement): { 
  before: HTMLElement | null; 
  after: HTMLElement | null; 
} | null {
  return null; // No longer used but kept for API compatibility
}