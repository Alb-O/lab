/**
 * Implements hover and playback UI controls for video elements
 */

/**
 * Sets up hover behavior and play/pause state classes for video elements
 */
export function setupVideoControls(): void {
  document.querySelectorAll('video').forEach(videoEl => {
    // Handle initial state
    if (videoEl.paused) {
      videoEl.classList.add('paused');
    }
    
    // Play/pause toggle
    videoEl.addEventListener('pause', () => {
      videoEl.classList.add('paused');
      videoEl.dispatchEvent(new Event('mouseleave'));
    });

    videoEl.addEventListener('play', () => {
      videoEl.classList.remove('paused');
    });

    // Hover toggles
    videoEl.addEventListener('mouseenter', () => videoEl.classList.add('hovered'));
    videoEl.addEventListener('mouseleave', () => videoEl.classList.remove('hovered'));
  });
}