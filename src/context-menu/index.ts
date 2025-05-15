import { Menu, Notice, TFile, MarkdownView, FileManager } from 'obsidian';
import { extractVideosFromMarkdownView, observeVideos } from '../video';
import { formatTimestamp } from '../timestamps/utils';
import { generateMarkdownLink } from 'obsidian-dev-utils/obsidian/Link';

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
      menu.addItem(item =>
        item
          .setIcon('link')
          .setTitle('Copy embed link at current time')
          .onClick(() => {
            // Get the current time of the video
            const currentTime = video.currentTime;
            const formattedTime = formatTimestamp(currentTime);
            
            // Get the path and file of the video
            const view = app.workspace.getActiveViewOfType(MarkdownView);
            if (!view) {
              new Notice('Cannot copy timestamp outside markdown view.');
              return;
            }
            const els = view.contentEl.querySelectorAll('video');
            const idx = Array.from(els).indexOf(video);
            let path: string;
            if (view.getMode() === 'preview') {
              path = video.dataset.timestampPath || '';
            } else {
              const videosMeta = extractVideosFromMarkdownView(view);
              if (idx < 0 || idx >= videosMeta.length) {
                new Notice('Video metadata not found.');
                return;
              }
              path = videosMeta[idx].path;
            }

            console.log('Video path:', path);
                         
            // Find the actual file via metadataCache or vault path
            const activeFile = app.workspace.getActiveFile();
            let file: TFile | null = null;
            if (activeFile) {
              const dest = app.metadataCache.getFirstLinkpathDest(path, activeFile.path);
              if (dest instanceof TFile) {
                file = dest;
                console.log('Resolved via metadataCache:', file);
              }
            }
            if (!file) {
              const normalized = path.replace(/\\/g, '/').replace(/^\//, '');
              const found = app.vault.getAbstractFileByPath(normalized);
              if (found instanceof TFile) {
                file = found;
                console.log('Resolved via vault:', file);
              }
            }
            if (!file) {
              new Notice(`File not found: ${path}`);
              return;
            }
            
            // Create a markdown link with timestamp
            const timestampParam = `#t=${currentTime}`;
            let linkText: string;
            
            if (file) {
              // If we found the actual file, use generateMarkdownLink
              linkText = generateMarkdownLink({
                app: app,
                targetPathOrFile: file,
                sourcePathOrFile: app.workspace.getActiveFile() || '',
                subpath: timestampParam,
                alias: formattedTime
              });
            } else {
              new Notice('File not found.');
              return;
            }
            
            // Copy to clipboard
            navigator.clipboard.writeText(linkText)
              .then(() => {
                new Notice(`Copied link with timestamp (${formattedTime}).`);
              })
              .catch(err => {
                console.error('Failed to copy link: ', err);
                new Notice('Failed to copy link to clipboard.');
              });
          })
      );

      menu.addItem(item =>
        item
          .setIcon('trash')
          .setTitle('Remove embed link')
          .onClick(async () => {
            const view = app.workspace.getActiveViewOfType(MarkdownView);
            if (!view) return;
            // prevent removal in preview (reading) mode
            if (view.getMode() === 'preview') {
              new Notice('Cannot remove while in reading view.');
              return;
            }
            const videos = extractVideosFromMarkdownView(view);

            // Match this <video> element to its metadata by index
            const els = view.contentEl.querySelectorAll('video');
            const idx = Array.from(els).indexOf(video);
            if (idx < 0 || idx >= videos.length) return;
            const target = videos[idx];

            // Remove only the specific embed link at position
            const { start, end } = target.position;
            const editor = view.editor;
            editor.replaceRange(
              '',
              { line: start.line, ch: start.col },
              { line: end.line, ch: end.col }
            );
            if (editor.getLine(start.line).trim() === '') {
              editor.replaceRange(
                '',
                { line: start.line, ch: 0 },
                { line: start.line + 1, ch: 0 }
              );
            }
          })
      );
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
