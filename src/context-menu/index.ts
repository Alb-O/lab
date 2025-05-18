import { Menu, Plugin } from 'obsidian';
import { VideoFragmentsSettings } from '../settings';
import { observeVideos } from '../video';
import { addOpenCommands } from './items/open';
import { addEmbedActionsCommands } from './items/embed-actions';
import { addSetCommands } from './items/set';
import { addSystemCommands } from './items/system';

// Track which elements already have context menus to prevent duplicates
const initializedElements = new WeakSet<HTMLVideoElement>();

/**
 * Sets up an Obsidian-native context menu on video elements.
 * Enables copying video links with fragments.
 */
export function setupVideoContextMenu(plugin: Plugin, settings: VideoFragmentsSettings, getAllRelevantDocuments: () => Document[]): () => void {
  // Clean up any previously initialized elements
  // It's important to call this to ensure handlers are removed before re-adding
  cleanupVideoContextMenu(getAllRelevantDocuments());

  const initContext = (video: HTMLVideoElement) => {
    // If the element is in initializedElements but the handler is missing,
    // it means it was cleaned up but not removed from the WeakSet.
    // We should re-initialize in this case.
    if (initializedElements.has(video) && video._videoContextMenuHandler) {
      return; // Already initialized and handler exists
    }

    // Create the handler function for the context menu
    const contextMenuHandler = (event: MouseEvent) => {
      event.preventDefault();
      const menu = new Menu();

      // Open
      addOpenCommands(menu, plugin, video);

      // Copy
      addEmbedActionsCommands(menu, plugin, settings, video);

      // Set
      addSetCommands(menu, plugin, settings, video);

      // System
      addSystemCommands(menu, plugin, video);

      menu.showAtPosition({ x: event.clientX, y: event.clientY });
    };

    // Store the handler on the element for later cleanup
    video._videoContextMenuHandler = contextMenuHandler;

    // Add the event listener
    video.addEventListener('contextmenu', contextMenuHandler);

    // Mark as initialized
    initializedElements.add(video);
    video.dataset.contextMenuInitialized = 'true';
  };

  // Observe all videos and initialize context menu once per element
  const cleanup = observeVideos(initContext, getAllRelevantDocuments);
  return cleanup;
}

/**
 * Clean up context menu handlers from all videos
 */
export function cleanupVideoContextMenu(documents: Document[]): void {
  documents.forEach(doc => {
    doc.querySelectorAll('video').forEach((video: HTMLVideoElement) => {
      if (video._videoContextMenuHandler) {
        video.removeEventListener('contextmenu', video._videoContextMenuHandler);
        delete video._videoContextMenuHandler;
        video.dataset.contextMenuInitialized = 'false';
        // Don't remove from initializedElements as we can't modify a WeakSet
      }
    });
  });
}
