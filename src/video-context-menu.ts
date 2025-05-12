import { App, Menu } from 'obsidian';
import { VideoDetector } from './video-detector';

/**
 * Sets up an Obsidian-native context menu on video elements.
 * Placeholder items currently do nothing.
 */
export function setupVideoContextMenu(app: App, videoDetector: VideoDetector): () => void {
  const initContext = (video: HTMLVideoElement) => {
    if (video.dataset.contextMenuInitialized === 'true') return;

    video.addEventListener('contextmenu', (event: MouseEvent) => {
      event.preventDefault();

      const menu = new Menu();
      menu.addItem(item =>
        item
          .setIcon('clock')
          .setTitle('Placeholder Action 1')
          .onClick(() => {
            // TODO: implement action
          })
      );
      menu.addItem(item =>
        item
          .setIcon('gear')
          .setTitle('Placeholder Action 2')
          .onClick(() => {
            // TODO: implement action
          })
      );
      menu.showAtPosition({ x: event.clientX, y: event.clientY });
    });

    video.dataset.contextMenuInitialized = 'true';
  };

  // Setup observer for new videos
  const observer = new MutationObserver(mutations => {
    let videoAdded = false;
    for (const mutation of mutations) {
      if (mutation.type === 'childList') {
        for (const node of Array.from(mutation.addedNodes)) {
          if (node instanceof HTMLVideoElement) {
            initContext(node);
            videoAdded = true;
          } else if (node instanceof Element) {
            const newVideos = node.querySelectorAll('video');
            if (newVideos.length > 0) {
              newVideos.forEach(initContext);
              videoAdded = true;
            }
          }
        }
      }
    }
  });
  
  // Start observing the document
  observer.observe(document.body, { childList: true, subtree: true });

  // Initialize existing videos
  document.querySelectorAll('video').forEach(initContext);
  
  // Return a cleanup function
  return () => {
    observer.disconnect();
  };
}
