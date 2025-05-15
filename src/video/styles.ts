/**
 * Clears custom timeline styling for the allowed segment.
 * @param videoEl The video element.
 */
interface CustomVideoElement extends HTMLVideoElement {
  _shadowStyle?: HTMLStyleElement;
  _debugOverlay?: HTMLElement;
  _fullscreenChangeHandler?: () => void;
}

interface CustomDocument extends Document {
  webkitFullscreenElement?: Element;
}

export function clearTimelineStyles(videoEl: HTMLVideoElement): void {
  const customVideoEl = videoEl as CustomVideoElement;
  // Remove injected style element if it exists
  if (customVideoEl.parentNode) {
    const container = customVideoEl.parentNode as HTMLElement;
    const styleEl = container.querySelector('.video-timestamps-style');
    if (styleEl) {
      styleEl.remove();
    }
  }

  // Remove shadow DOM injected style if it exists
  try {
    if (customVideoEl.shadowRoot && customVideoEl._shadowStyle) {
      customVideoEl._shadowStyle.remove();
      delete customVideoEl._shadowStyle;
    }
  } catch (e) {
    // Ignore shadow DOM access errors
  }

  // Remove debug overlay if it exists
  if (customVideoEl._debugOverlay) {
    customVideoEl._debugOverlay.remove();
    delete customVideoEl._debugOverlay;
  }

  // Remove any fullscreen change listeners
  if (customVideoEl._fullscreenChangeHandler) {
    document.removeEventListener('fullscreenchange', customVideoEl._fullscreenChangeHandler);
    document.removeEventListener('webkitfullscreenchange', customVideoEl._fullscreenChangeHandler);
    delete customVideoEl._fullscreenChangeHandler;
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
 * @param initialDuration The initial reported duration of the video.
 */
export function updateTimelineStyles(videoEl: HTMLVideoElement, startTime: number, endTime: number, initialDuration: number): void {
  const customVideoEl = videoEl as CustomVideoElement;
  const customDocument = document as CustomDocument;

  const performStyling = (currentDuration: number) => {
    // Ensure styles are clear before applying new ones
    clearTimelineStyles(customVideoEl);

    if (!currentDuration || !isFinite(currentDuration) || currentDuration <= 0) {
      return;
    }

    const startPercent = Math.max(0, Math.min(100, (startTime / currentDuration) * 100));
    const endPercent = endTime === Infinity ? 100 : Math.max(0, Math.min(100, (endTime / currentDuration) * 100));
    videoEl.dataset.startTimePercent = startPercent.toFixed(2);
    videoEl.dataset.endTimePercent = endPercent.toFixed(2);

    const videoId = `video-ts-${Date.now()}-${Math.floor(Math.random() * 1000)}`;
    videoEl.classList.add(videoId);

    // Remove any existing fullscreen listeners
    if (customVideoEl._fullscreenChangeHandler) {
      document.removeEventListener('fullscreenchange', customVideoEl._fullscreenChangeHandler);
      document.removeEventListener('webkitfullscreenchange', customVideoEl._fullscreenChangeHandler);
    }

    const fullscreenChangeHandler = () => {
        performStyling(customVideoEl.duration);
    };

    customVideoEl._fullscreenChangeHandler = fullscreenChangeHandler;
    document.addEventListener('fullscreenchange', fullscreenChangeHandler);
    document.addEventListener('webkitfullscreenchange', fullscreenChangeHandler);

    const isFullscreen = customDocument.fullscreenElement === customVideoEl || customDocument.webkitFullscreenElement === customVideoEl;
    let trackLeft = 16, trackWidth = 0, totalWidth = 0;

    try {
      if (isFullscreen) {
        const fsControlsScale = 2;
        trackLeft = 16 * fsControlsScale;
        trackWidth = window.innerWidth - (32 * fsControlsScale);
        totalWidth = window.innerWidth;
      }
    } catch (e) {
      trackLeft = 16;
      trackWidth = 0;
      totalWidth = 0;
    }

    const thumbRadius = 6;
    let effectiveTrackLeft = trackLeft + thumbRadius;
    let effectiveTrackWidth = trackWidth ? trackWidth - 2 * thumbRadius : 0;

    if (!effectiveTrackWidth || !totalWidth || totalWidth <= 0) { // Added totalWidth <= 0 check
      const videoWidth = videoEl.offsetWidth;
      totalWidth = videoWidth > 0 ? videoWidth : 300; // Fallback totalWidth if videoWidth is 0
      const defaultPadding = 16;
      effectiveTrackLeft = defaultPadding + thumbRadius;
      effectiveTrackWidth = totalWidth - 2 * (defaultPadding + thumbRadius);
      if (effectiveTrackWidth <= 0) effectiveTrackWidth = totalWidth / 2; // Ensure positive width
    }

    const fullStartPercent = totalWidth
      ? ((effectiveTrackLeft + effectiveTrackWidth * (startPercent / 100)) / totalWidth) * 100
      : startPercent;
    const fullEndPercent = totalWidth
      ? ((effectiveTrackLeft + effectiveTrackWidth * (endPercent / 100)) / totalWidth) * 100
      : endPercent;

    const epsilon = totalWidth ? (thumbRadius / totalWidth) * 100 : 0.1; // Ensure epsilon is small but non-zero
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
      // Fallback if parentNode is null (e.g. video not in DOM yet, though unlikely here)
      document.head.appendChild(styleEl);
    }

    setTimeout(() => {
      try {
        if (customVideoEl.shadowRoot) {
          const timeline = customVideoEl.shadowRoot.querySelector('input[pseudo="-webkit-media-controls-timeline"]');
          if (timeline) {
            const htmlTimeline = timeline as HTMLElement;
            htmlTimeline.style.display = 'none';
            void htmlTimeline.offsetHeight; // Trigger reflow
            htmlTimeline.style.display = '';
          }
        }
      } catch (e) {
      }
    }, 100);

  };

  // Try with initialDuration if provided and valid
  if (initialDuration && isFinite(initialDuration) && initialDuration > 0) {
    performStyling(initialDuration);
  }
}

function getCssVar(name: string): string {
  return getComputedStyle(document.body).getPropertyValue(name).trim() ?? '';
}