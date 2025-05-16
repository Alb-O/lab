import { Menu, Notice, TFile, MarkdownView, FileManager, normalizePath } from 'obsidian';
import { extractVideosFromMarkdownView, observeVideos } from '../video';
import { formatTimestamp } from '../timestamps/utils';
import { generateMarkdownLink } from 'obsidian-dev-utils/obsidian/Link';
import { getVideoLinkDetails } from './link-retriever'; // Import the new function

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
            
            const linkDetails = getVideoLinkDetails(app, video);

            if (!linkDetails) {
              new Notice('Cannot copy link: View type not supported or active leaf not found.');
              return;
            }

            const { targetFile, sourcePathForLink, originalVideoSrcForNotice, isExternalFileUrl, externalFileUrl } = linkDetails;

            if (!targetFile && !isExternalFileUrl) {
              new Notice(`Video file not found. Source: ${originalVideoSrcForNotice || 'unknown'}`);
              return;
            }
            
            let linkText: string;

            if (isExternalFileUrl && externalFileUrl) {
                const baseSrc = externalFileUrl.split('#')[0];
                const newSrcWithTimestamp = `${baseSrc}#t=${currentTime}`;
                const excludedAttributes = [
                    'data-controls-initialized', 'data-timestamp-path', 'data-context-menu-initialized',
                    'data-start-time', 'data-end-time', 'data-start-time-percent', 'data-end-time-percent',
                    'data-reached-end', 'data-seeked-past-end', 'data-auto-resume', 'data-should-auto-play',
                    'data-user-paused', 'data-is-seeking'
                ];
                
                let attributesString = "";
                for (const attr of Array.from(video.attributes)) {
                    const attrNameLower = attr.name.toLowerCase();
                    if (excludedAttributes.includes(attrNameLower) || attrNameLower.startsWith('video-ts-')) { // Exclude dynamic class
                        continue;
                    }
                    if (attrNameLower === 'class' && attr.value === '') {
                        continue;
                    }

                    if (attrNameLower === 'src') {
                        attributesString += ` src="${newSrcWithTimestamp}"`;
                    } else {
                        if (attr.value === '') { // Boolean attribute
                            attributesString += ` ${attr.name}`;
                        } else {
                            attributesString += ` ${attr.name}="${attr.value}"`;
                        }
                    }
                }
                // Ensure src is present if it wasn't an attribute but derived from externalFileUrl
                if (!video.hasAttribute('src') && !attributesString.includes(' src=')) {
                    attributesString += ` src="${newSrcWithTimestamp}"`;
                }
                linkText = `<video${attributesString}></video>`;
            } else if (targetFile) {
                const timestampParam = `#t=${currentTime}`;
                linkText = generateMarkdownLink({
                  app: app,
                  targetPathOrFile: targetFile,
                  sourcePathOrFile: sourcePathForLink,
                  subpath: timestampParam,
                  alias: formattedTime,
                  isEmbed: true
                });
            } else {
                new Notice('Could not determine link type.');
                return;
            }
            
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
          .setIcon('link')
          .setTitle('Copy embed link')
          .onClick(() => {
            const linkDetails = getVideoLinkDetails(app, video);

            if (!linkDetails) {
              new Notice('Cannot copy link: View type not supported or active leaf not found.');
              return;
            }

            const { targetFile, sourcePathForLink, originalVideoSrcForNotice, isExternalFileUrl, externalFileUrl } = linkDetails;

            if (!targetFile && !isExternalFileUrl) {
              new Notice(`Video file not found. Source: ${originalVideoSrcForNotice || 'unknown'}`);
              return;
            }

            let linkText: string;

            if (isExternalFileUrl && externalFileUrl) {
                // This line strips any existing fragment (like #t=...) from the externalFileUrl
                const baseSrc = externalFileUrl.split('#')[0]; 
                const excludedAttributes = [
                    'data-controls-initialized', 'data-timestamp-path', 'data-context-menu-initialized',
                    'data-start-time', 'data-end-time', 'data-start-time-percent', 'data-end-time-percent',
                    'data-reached-end', 'data-seeked-past-end', 'data-auto-resume', 'data-should-auto-play',
                    'data-user-paused', 'data-is-seeking'
                ];
                
                let attributesString = "";
                for (const attr of Array.from(video.attributes)) {
                    const attrNameLower = attr.name.toLowerCase();
                    if (excludedAttributes.includes(attrNameLower) || attrNameLower.startsWith('video-ts-')) { // Exclude dynamic class
                        continue;
                    }
                    if (attrNameLower === 'class' && attr.value === '') {
                        continue;
                    }
                    
                    if (attrNameLower === 'src') {
                        // The src attribute is set to the baseSrc, which has no timestamp
                        attributesString += ` src="${baseSrc}"`; 
                    } else {
                        if (attr.value === '') { // Boolean attribute
                            attributesString += ` ${attr.name}`;
                        } else {
                            attributesString += ` ${attr.name}="${attr.value}"`;
                        }
                    }
                }
                // Ensure src is present if it wasn't an attribute but derived from externalFileUrl
                if (!video.hasAttribute('src') && !attributesString.includes(' src=')) {
                     attributesString += ` src="${baseSrc}"`; // Use baseSrc here as well
                }
                linkText = `<video${attributesString}></video>`;
            } else if (targetFile) {
                linkText = generateMarkdownLink({
                  app: app,
                  targetPathOrFile: targetFile,
                  sourcePathOrFile: sourcePathForLink,
                  isEmbed: true 
                  // No subpath is provided, so no timestamp for internal files either
                });
            } else {
                new Notice('Could not determine link type.');
                return;
            }
            
            navigator.clipboard.writeText(linkText)
              .then(() => {
                new Notice('Copied embed link.');
              })
              .catch(err => {
                console.error('Failed to copy link: ', err);
                new Notice('Failed to copy link to clipboard.');
              });
          })
      );

      menu.addItem(item =>
        item
          .setIcon('file-video') // Or another appropriate icon
          .setTitle('Open video in new tab')
          .onClick(() => {
            const linkDetails = getVideoLinkDetails(app, video);

            if (!linkDetails) {
              new Notice('Cannot open video: View type not supported or active leaf not found.');
              return;
            }

            const { targetFile, originalVideoSrcForNotice, isExternalFileUrl, externalFileUrl } = linkDetails;

            if (!targetFile && !isExternalFileUrl) {
              new Notice(`Video file not found. Source: ${originalVideoSrcForNotice || 'unknown'}`);
              return;
            }
            
            if (isExternalFileUrl && externalFileUrl) {
                window.open(externalFileUrl.split('#')[0]);
            } else if (targetFile) {
                app.workspace.openLinkText(targetFile.path, '', true); 
            } else {
                new Notice('Could not determine video to open.');
            }
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
