import { Menu, Notice, TFile, MarkdownView, FileManager, normalizePath } from 'obsidian';
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
            const currentTime = video.currentTime;
            const formattedTime = formatTimestamp(currentTime);
            
            const activeLeaf = app.workspace.activeLeaf;
            if (!activeLeaf) {
              new Notice('No active leaf found.');
              return;
            }

            let targetFile: TFile | null = null;
            let sourcePathForLink: string = '';
            let originalVideoSrcForNotice: string | null = video.dataset.timestampPath || video.currentSrc || video.src;

            if (activeLeaf.view instanceof MarkdownView) {
              const mdView = activeLeaf.view;
              sourcePathForLink = mdView.file?.path || '';

              if (mdView.getMode() === 'preview') {
                const currentVideoSrc = video.dataset.timestampPath || video.currentSrc || video.src;
                if (currentVideoSrc) {
                    if (currentVideoSrc.startsWith('app://')) {
                        try {
                            const url = new URL(currentVideoSrc);
                            let absPathFromUrl = decodeURIComponent(url.pathname); // Already decoded by URL constructor

                            // Normalize: remove leading slash if it's not part of a Windows drive, and use forward slashes
                            if (absPathFromUrl.startsWith('/') && absPathFromUrl.length > 1 && absPathFromUrl[1] !== ':') {
                                absPathFromUrl = absPathFromUrl.substring(1);
                            }
                            absPathFromUrl = normalizePath(absPathFromUrl); // Obsidian's normalizePath handles separators

                            const vaultBasePath = normalizePath(app.vault.adapter.getBasePath());
                            
                            if (absPathFromUrl.toLowerCase().startsWith(vaultBasePath.toLowerCase())) {
                                const relativePath = normalizePath(absPathFromUrl.substring(vaultBasePath.length + (vaultBasePath.endsWith('/') ? 0 : 1) ));
                                targetFile = app.vault.getFileByPath(relativePath);
                            }
                            
                            if (!targetFile) {
                                 console.warn(`VideoTimestamps: Could not find TFile for app:// URL. Derived relative path: ${absPathFromUrl.substring(vaultBasePath.length + 1)}. Original src: ${currentVideoSrc}`);
                            }
                        } catch (e) {
                            console.error('VideoTimestamps: Error parsing app:// URL for video path:', currentVideoSrc, e);
                        }
                    } else { // Not an app:// URL, try resolving normally (e.g. relative path or linktext from dataset)
                        const pathFromDataset = currentVideoSrc.split('#')[0];
                        const resolvedFile = app.metadataCache.getFirstLinkpathDest(pathFromDataset, sourcePathForLink);
                        if (resolvedFile instanceof TFile) {
                            targetFile = resolvedFile;
                        } else {
                            const normalizedDirectPath = normalizePath(pathFromDataset);
                            const foundFile = app.vault.getFileByPath(normalizedDirectPath);
                            if (foundFile instanceof TFile) {
                                targetFile = foundFile;
                            }
                        }
                    }
                }
              } else { // Source or Live Preview mode
                const videosMeta = extractVideosFromMarkdownView(mdView);
                const els = mdView.contentEl.querySelectorAll('video');
                const idx = Array.from(els).indexOf(video);
                if (idx >= 0 && idx < videosMeta.length) {
                  const videoMetaPath = videosMeta[idx].path; 
                  const resolvedFile = app.vault.getAbstractFileByPath(videoMetaPath); // .path is already vault-relative
                  if (resolvedFile instanceof TFile) {
                    targetFile = resolvedFile;
                  }
                }
                if (!targetFile) {
                    new Notice('Video metadata not found or file unresolved in editor view.');
                    return;
                }
              }
            } else if (activeLeaf.view.getViewType() === 'video' && activeLeaf.view.file instanceof TFile) {
              targetFile = activeLeaf.view.file;
              sourcePathForLink = ''; // For vault-absolute link
            } else {
              new Notice('Cannot copy timestamp: Not a Markdown or direct video view.');
              return;
            }

            if (!targetFile) {
              new Notice(`Video file not found. Source: ${originalVideoSrcForNotice || 'unknown'}`);
              return;
            }
            
            const timestampParam = `#t=${currentTime}`;
            const linkText = generateMarkdownLink({
              app: app,
              targetPathOrFile: targetFile,
              sourcePathOrFile: sourcePathForLink,
              subpath: timestampParam,
              alias: formattedTime,
              isEmbed: true
            });
            
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
            if (!view) {
              new Notice('Removing embed links only works from a Markdown note.');
              return;
            }
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
