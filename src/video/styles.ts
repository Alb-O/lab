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
  // Remove any fullscreen change listeners
  if ((videoEl as any)._fullscreenChangeHandler) {
    document.removeEventListener('fullscreenchange', (videoEl as any)._fullscreenChangeHandler);
    document.removeEventListener('webkitfullscreenchange', (videoEl as any)._fullscreenChangeHandler);
    delete (videoEl as any)._fullscreenChangeHandler;
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

  const startPercent = Math.max(0, Math.min(100, (startTime / duration) * 100));
  const endPercent = endTime === Infinity ? 100 : Math.max(0, Math.min(100, (endTime / duration) * 100));
  videoEl.dataset.startTimePercent = startPercent.toFixed(2);
  videoEl.dataset.endTimePercent = endPercent.toFixed(2);

  const videoId = `video-ts-${Date.now()}-${Math.floor(Math.random() * 1000)}`;
  videoEl.classList.add(videoId);
  
  // Remove any existing fullscreen listeners
  if ((videoEl as any)._fullscreenChangeHandler) {
    document.removeEventListener('fullscreenchange', (videoEl as any)._fullscreenChangeHandler);
    document.removeEventListener('webkitfullscreenchange', (videoEl as any)._fullscreenChangeHandler);
  }
  // Try to get the track bar's left offset and width from the shadow DOM
  const isFullscreen = document.fullscreenElement === videoEl || (document as any).webkitFullscreenElement === videoEl;
  console.debug('[VideoTimestamps] Calculating track metrics - fullscreen:', isFullscreen);
  let trackLeft = 16, trackWidth = 0, totalWidth = 0;
  
  // Set up fullscreen change listener to re-apply styles when fullscreen state changes
  const fullscreenChangeHandler = () => {
    const newIsFullscreen = document.fullscreenElement === videoEl || (document as any).webkitFullscreenElement === videoEl;
    console.debug('[VideoTimestamps] Fullscreen change detected:', newIsFullscreen);
    // Short delay to allow browser to stabilize the fullscreen state
    setTimeout(() => {
      updateTimelineStyles(videoEl, startTime, endTime, duration);
    }, 150);
  };
  
  // Store the handler on the video element for later removal
  (videoEl as any)._fullscreenChangeHandler = fullscreenChangeHandler;
  document.addEventListener('fullscreenchange', fullscreenChangeHandler);
  document.addEventListener('webkitfullscreenchange', fullscreenChangeHandler);
  
  try {
    if (isFullscreen) {
      // Fullscreen mode requires different metrics
      const fsControlsScale = 1.8; // Controls are approx. 1.8x larger in fullscreen
      trackLeft = 16 * fsControlsScale;
      trackWidth = window.innerWidth - (32 * fsControlsScale);
      totalWidth = window.innerWidth;
      console.debug('[VideoTimestamps] Using fullscreen metrics:', trackLeft, trackWidth, totalWidth);
    } else if (videoEl.shadowRoot) {
      const track = videoEl.shadowRoot.querySelector('div[pseudo="-internal-media-controls-segmented-track"]') as HTMLElement;
      if (track) {
        const rect = track.getBoundingClientRect();
        const parentRect = (track.parentElement as HTMLElement)?.getBoundingClientRect();
        trackLeft = rect.left - (parentRect?.left ?? 0);
        trackWidth = rect.width;
        totalWidth = (track.parentElement as HTMLElement)?.offsetWidth ?? (trackWidth + 2 * trackLeft);
      }
    }
  } catch (e) {
    trackLeft = 16;
    trackWidth = 0;
    totalWidth = 0;
  }

  // Account for 6px thumb radius on both sides
  const thumbRadius = 6;
  let effectiveTrackLeft = trackLeft + thumbRadius;
  let effectiveTrackWidth = trackWidth ? trackWidth - 2 * thumbRadius : 0;

  // Fallback when track dimensions couldn't be measured
  if (!effectiveTrackWidth || !totalWidth) {
    const videoWidth = videoEl.offsetWidth;
    totalWidth = videoWidth;
    const defaultPadding = 16; // browser control side padding
    effectiveTrackLeft = defaultPadding + thumbRadius;
    effectiveTrackWidth = videoWidth - 2 * (defaultPadding + thumbRadius);
  }

  // Compute full control-relative gradient stops in percent
  const fullStartPercent = totalWidth
    ? ((effectiveTrackLeft + effectiveTrackWidth * (startPercent / 100)) / totalWidth) * 100
    : startPercent;
  const fullEndPercent = totalWidth
    ? ((effectiveTrackLeft + effectiveTrackWidth * (endPercent / 100)) / totalWidth) * 100
    : endPercent;

  // Prevent true edge snap: add small epsilon to account for padding gap
  const epsilon = (thumbRadius / totalWidth) * 100;
  const clampStart = Math.max(epsilon, Math.min(100 - epsilon, fullStartPercent));
  const clampEnd = Math.max(epsilon, Math.min(100 - epsilon, fullEndPercent));

  const bgColor = getCssVar('--video-ts-timeline-bg') || 'rgba(240,50,50,0)';
  const fgColor = getCssVar('--video-ts-timeline-playable') || 'rgba(76,175,80,0.8)';
  const cssContent = `
    /* Timeline styling for video with allowed range ${startPercent.toFixed(2)}%–${endPercent.toFixed(2)}% */
    .${videoId}::-webkit-media-controls-timeline {
      background-origin: content-box !important;
      background-clip: content-box !important;
      background: linear-gradient(to right,
        ${bgColor} 0%,
        ${bgColor} ${clampStart.toFixed(2)}%,
        ${fgColor} ${clampStart.toFixed(2)}%,
        ${fgColor} ${clampEnd.toFixed(2)}%,
        ${bgColor} ${clampEnd.toFixed(2)}%,
        ${bgColor} 100%
      ) !important;
    }
  `;

  const styleEl = document.createElement('style');
  styleEl.className = 'video-timestamps-style';
  styleEl.textContent = cssContent;
  if (videoEl.parentNode) {
    videoEl.parentNode.insertBefore(styleEl, videoEl.nextSibling);
  } else {
    document.head.appendChild(styleEl);
  }

  setTimeout(() => {
    try {
      if (videoEl.shadowRoot) {
        const timeline = videoEl.shadowRoot.querySelector('input[pseudo="-webkit-media-controls-timeline"]');
        if (timeline) {
          const htmlTimeline = timeline as HTMLElement;
          htmlTimeline.style.display = 'none';
          void htmlTimeline.offsetHeight;
          htmlTimeline.style.display = '';
        }
      }
    } catch (e) {}
  }, 100);

  console.debug('[VideoTimestamps] Applied timeline styling for range', startTime, 'to', endTime);
}

/**
 * Dump shadow DOM structure to help debug styling issues
 */
export function dumpShadowDomStructure(videoEl: HTMLVideoElement): void {
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

function getCssVar(name: string): string {
    return getComputedStyle(document.body).getPropertyValue(name).trim() ?? '';
}