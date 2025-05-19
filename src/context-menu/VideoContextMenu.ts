import { Menu, Plugin } from 'obsidian';
import { VideoFragmentsSettings } from '@settings';
import { observeElements } from '@observer';
import { addOpenCommands, addEmbedActionsCommands, addSetCommands, addSystemCommands } from '@context-menu/items';

export class VideoContextMenu {
  private plugin: Plugin;
  private settings: VideoFragmentsSettings;
  private getAllRelevantDocuments: () => Document[];
  private initializedElements = new WeakSet<HTMLVideoElement>();
  private cleanupFn: (() => void) | null = null;

  constructor(plugin: Plugin, settings: VideoFragmentsSettings, getAllRelevantDocuments: () => Document[]) {
    this.plugin = plugin;
    this.settings = settings;
    this.getAllRelevantDocuments = getAllRelevantDocuments;
  }

  /**
   * Set up the context menu for all video elements.
   * Returns a cleanup function.
   */
  public setup(): () => void {
    this.cleanup();
    const initContext = (video: HTMLVideoElement) => {
      if (this.initializedElements.has(video) && video._videoContextMenuHandler) {
        return;
      }
      const contextMenuHandler = (event: MouseEvent) => {
        event.preventDefault();
        const menu = new Menu();
        addOpenCommands(menu, this.plugin, video);
        addEmbedActionsCommands(menu, this.plugin, this.settings, video);
        addSetCommands(menu, this.plugin, this.settings, video);
        addSystemCommands(menu, this.plugin, video);
        menu.showAtPosition({ x: event.clientX, y: event.clientY });
      };
      video._videoContextMenuHandler = contextMenuHandler;
      video.addEventListener('contextmenu', contextMenuHandler);
      this.initializedElements.add(video);
      video.dataset.contextMenuInitialized = 'true';
    };
    this.cleanupFn = observeElements(this.getAllRelevantDocuments, 'video', initContext);
    return this.cleanupFn;
  }

  /**
   * Clean up all context menu handlers from all videos.
   */
  public cleanup(): void {
    this.getAllRelevantDocuments().forEach(doc => {
      doc.querySelectorAll('video').forEach((video: HTMLVideoElement) => {
        if (video._videoContextMenuHandler) {
          video.removeEventListener('contextmenu', video._videoContextMenuHandler);
          delete video._videoContextMenuHandler;
          video.dataset.contextMenuInitialized = 'false';
        }
      });
    });
    if (this.cleanupFn) {
      this.cleanupFn();
      this.cleanupFn = null;
    }
  }
}
