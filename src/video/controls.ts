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
  
  // Mark as initialized
  videoEl.dataset.controlsInitialized = 'true';
}

/**
 * Clears custom timeline styling for the allowed segment.
 * @param videoEl The video element.
 */
export function clearTimelineStyles(videoEl: HTMLVideoElement): void {
  // Remove injected style element if it exists
  if (videoEl.parentNode) {
    const container = videoEl.parentNode as HTMLElement;
    const styleEl = container.querySelector('.video-timestamps-style');
    if (styleEl) {
      styleEl.remove();
    }
  }
  
  // Remove shadow DOM injected style if it exists
  try {
    if (videoEl.shadowRoot && (videoEl as any)._shadowStyle) {
      (videoEl as any)._shadowStyle.remove();
      delete (videoEl as any)._shadowStyle;
    }
  } catch (e) {
    // Ignore shadow DOM access errors
  }
  
  // Remove debug overlay if it exists
  if ((videoEl as any)._debugOverlay) {
    (videoEl as any)._debugOverlay.remove();
    delete (videoEl as any)._debugOverlay;
  }
  
  // Remove any unique class we added
  Array.from(videoEl.classList).forEach(cls => {
    if (cls.startsWith('video-ts-')) {
      videoEl.classList.remove(cls);
    }
  });
  
  // Remove custom data attributes
  delete videoEl.dataset.startTimePercent;
  delete videoEl.dataset.endTimePercent;
}

/**
 * Updates the video timeline to visually represent the allowed segment.
 * @param videoEl The video element.
 * @param startTime The start time of the restricted segment.
 * @param endTime The end time of the restricted segment (can be Infinity).
 * @param duration The total duration of the video.
 */
export function updateTimelineStyles(videoEl: HTMLVideoElement, startTime: number, endTime: number, duration: number): void {
  // Clear any existing styles first
  clearTimelineStyles(videoEl);
  
  if (!duration || !isFinite(duration) || duration <= 0) {
    console.debug('[VideoTimestamps] Cannot style timeline - invalid duration');
    return;
  }

  // Calculate percentages for CSS
  const startPercent = Math.max(0, Math.min(100, (startTime / duration) * 100));
  const endPercent = endTime === Infinity ? 100 : Math.max(0, Math.min(100, (endTime / duration) * 100));
  
  // Store percentages as data attributes for potential JavaScript usage
  videoEl.dataset.startTimePercent = startPercent.toFixed(2);
  videoEl.dataset.endTimePercent = endPercent.toFixed(2);
  
  // Create unique ID for this video element to target CSS
  const videoId = `video-ts-${Date.now()}-${Math.floor(Math.random() * 1000)}`;
  videoEl.classList.add(videoId);
  
  // Also create external styles as fallback
  const cssContent = `
    /* Timeline styling for video with allowed range ${startPercent.toFixed(2)}% to ${endPercent.toFixed(2)}% */
    .${videoId} {
      --ts-start-percent: ${startPercent}%;
      --ts-end-percent: ${endPercent}%;
    }
    
    /* Chrome/Edge approach: multi-layered targeting */
    .${videoId}::-webkit-media-controls-timeline {
      background: linear-gradient(to right, 
        rgba(240, 50, 50, 0.8) 0%, 
        rgba(240, 50, 50, 0.8) var(--ts-start-percent), 
        rgba(76, 175, 80, 0.8) var(--ts-start-percent), 
        rgba(76, 175, 80, 0.8) var(--ts-end-percent), 
        rgba(240, 50, 50, 0.8) var(--ts-end-percent), 
        rgba(240, 50, 50, 0.8) 100%) !important;
    }
  `;
  
  // Create and inject style element next to the video
  const styleEl = document.createElement('style');
  styleEl.className = 'video-timestamps-style';
  styleEl.textContent = cssContent;
  
  // Add to parent if possible, otherwise add directly to document head
  if (videoEl.parentNode) {
    videoEl.parentNode.insertBefore(styleEl, videoEl.nextSibling);
  } else {
    document.head.appendChild(styleEl);
  }
  
  // Try to force a reflow of the timeline
  setTimeout(() => {
    try {
      if (videoEl.shadowRoot) {
        const timeline = videoEl.shadowRoot.querySelector('input[pseudo="-webkit-media-controls-timeline"]');
        if (timeline) {
          // Force a style recalculation - cast to HTMLElement properly
          const htmlTimeline = timeline as HTMLElement;
          htmlTimeline.style.display = 'none';
          void htmlTimeline.offsetHeight; // Trigger reflow
          htmlTimeline.style.display = '';
        }
      }
    } catch (e) {
      // Ignore errors accessing shadow DOM
    }
  }, 100);
  
  console.debug('[VideoTimestamps] Applied timeline styling for range', startTime, 'to', endTime);
}

/**
 * Dump shadow DOM structure to help debug styling issues
 */
function dumpShadowDomStructure(videoEl: HTMLVideoElement): void {
  try {
    if (!videoEl.shadowRoot) {
      console.debug('[VideoTimestamps] No shadow root available for inspection');
      return;
    }
    
    const timeline = videoEl.shadowRoot.querySelector('input[pseudo="-webkit-media-controls-timeline"]');
    console.debug('[VideoTimestamps] Timeline element:', timeline);
    
    if (timeline) {
      console.debug('[VideoTimestamps] Timeline attributes:', 
                  Array.from(timeline.attributes).map(attr => `${attr.name}="${attr.value}"`).join(' '));
                  
      if (timeline.shadowRoot) {
        console.debug('[VideoTimestamps] Timeline has its own shadow root:', timeline.shadowRoot);
        const trackSegments = timeline.shadowRoot.querySelectorAll('div[pseudo]');
        console.debug('[VideoTimestamps] Track segments:', trackSegments, 
                    Array.from(trackSegments).map(el => el.getAttribute('pseudo')));
      } else {
        console.debug('[VideoTimestamps] Timeline does not have its own shadow root');
      }
    }
    
    // Log all shadow elements with pseudo attributes
    const allPseudoElements = videoEl.shadowRoot.querySelectorAll('[pseudo]');
    console.debug('[VideoTimestamps] All pseudo elements in shadow DOM:', 
                Array.from(allPseudoElements).map(el => `${el.tagName}[pseudo="${el.getAttribute('pseudo')}"]`));
  } catch (e) {
    console.debug('[VideoTimestamps] Error inspecting shadow DOM:', e);
  }
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