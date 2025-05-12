import { Menu, Notice, TFile } from 'obsidian';
import { formatTimestamp } from './utils';
import { generateMarkdownLink } from 'obsidian-dev-utils/obsidian/Link';

/**
 * Sets up an Obsidian-native context menu on video elements.
 * Enables copying video links with timestamps.
 */
export function setupVideoContextMenu(app: any): () => void {
  const initContext = (video: HTMLVideoElement) => {
    if (video.dataset.contextMenuInitialized === 'true') return;

    video.addEventListener('contextmenu', (event: MouseEvent) => {
      event.preventDefault();

      const menu = new Menu();
      menu.addItem(item =>
        item
          .setIcon('link')
          .setTitle('Copy embed link at current time')
          .onClick(() => {            // Get the current time of the video
            const currentTime = video.currentTime;
            const formattedTime = formatTimestamp(currentTime);
            
            // Get the path and file of the video
            const path = video.dataset.timestampPath || 
                         video.src.split('/').pop() || 
                         'video';
                         
            // Find the actual file if possible
            let file: TFile | null = null;
            
            try {
              file = this.app.vault.getFileByPath(path);
            } catch (error) {
              console.error('Error finding video file:', error);
            }
            
            // Create a markdown link with timestamp
            const timestampParam = `#t=${currentTime}`;
            let linkText: string;
            
            if (file) {
              // If we found the actual file, use generateMarkdownLink
              linkText = generateMarkdownLink({
                app: this.app,
                targetPathOrFile: file,
                sourcePathOrFile: this.app.workspace.getActiveFile() || '',
                subpath: timestampParam,
                alias: formattedTime
              });
            } else {
              // Fallback to simple wiki link if file not found
              linkText = `![[${path}#${timestampParam}|${formattedTime}]]`;
            }
            
            // Copy to clipboard
            navigator.clipboard.writeText(linkText)
              .then(() => {
                new Notice(`Copied link with timestamp (${formattedTime})`);
              })
              .catch(err => {
                console.error('Failed to copy link: ', err);
                new Notice('Failed to copy link to clipboard');
              });
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
