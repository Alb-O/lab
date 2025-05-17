import { Menu } from 'obsidian';
import { observeVideos } from '../video';
import { addOpenLink, addOpenInNewTab, addOpenToRight, addOpenInNewWindow } from './items/open';
import { addCopyEmbedLink, addCopyEmbedAtCurrentTime } from './items/copy';
import { addRemoveEmbedLink } from './items/remove';

// Track which elements already have context menus to prevent duplicates
const initializedElements = new WeakSet<HTMLVideoElement>();

/**
 * Sets up an Obsidian-native context menu on video elements.
 * Enables copying video links with timestamps.
 */
export function setupVideoContextMenu(app: any): () => void {
  // Clean up any previously initialized elements
  cleanupVideoContextMenu();
  
  const initContext = (video: HTMLVideoElement) => {
    // Skip if already initialized
    if (initializedElements.has(video)) return;

    // Create the handler function for the context menu
    const contextMenuHandler = (event: MouseEvent) => {
      event.preventDefault();

      const menu = new Menu();

      addOpenLink(menu, app, video);
      addOpenInNewTab(menu, app, video);
      addOpenToRight(menu, app, video);
      addOpenInNewWindow(menu, app, video);

      menu.addSeparator();

      addCopyEmbedLink(menu, app, video);
      addCopyEmbedAtCurrentTime(menu, app, video);

      menu.addSeparator();

      addRemoveEmbedLink(menu, app, video);

      menu.showAtPosition({ x: event.clientX, y: event.clientY });
    };
    
    // Store the handler on the element for later cleanup
    (video as any)._videoContextMenuHandler = contextMenuHandler;

    // Add the event listener
    video.addEventListener('contextmenu', contextMenuHandler);
    
    // Mark as initialized
    initializedElements.add(video);
    video.dataset.contextMenuInitialized = 'true';
  };

  // Observe all videos and initialize context menu once per element
  const cleanup = observeVideos(initContext);
  return cleanup;
}

/**
 * Clean up context menu handlers from all videos
 */
export function cleanupVideoContextMenu(): void {
  document.querySelectorAll('video').forEach((video: HTMLVideoElement) => {
    if ((video as any)._videoContextMenuHandler) {
      video.removeEventListener('contextmenu', (video as any)._videoContextMenuHandler);
      delete (video as any)._videoContextMenuHandler;
      video.dataset.contextMenuInitialized = 'false';
      // Don't remove from initializedElements as we can't modify a WeakSet
    }
  });
}
