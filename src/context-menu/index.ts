import { Menu, Notice, TFile, MarkdownView } from 'obsidian';
import { formatTimestamp, extractVideosFromMarkdownView, observeVideos } from '../utils';
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
      menu.addItem(item =>
        item
          .setIcon('trash')
          .setTitle('Remove embed link')
          .onClick(async () => {
            // Collect videos from current markdown view
            const view = app.workspace.getActiveViewOfType(MarkdownView);
            if (!view) return;
            const videos = extractVideosFromMarkdownView(view);
            // Find the VideoWithTimestamp matching this video element by path
            const target = videos.find(v => v.path === video.dataset.timestampPath);
            if (!target) return;
            // Remove only the specific embed link at position
            const { start, end } = target.position;
            const editor = view.editor;
            editor.replaceRange(
              '',
              { line: start.line, ch: start.col },
              { line: end.line, ch: end.col }
            );
            new Notice('Removed video embed link');
          })
      );
      menu.showAtPosition({ x: event.clientX, y: event.clientY });
    });

    video.dataset.contextMenuInitialized = 'true';
  };

  // Observe all videos and initialize context menu once per element
  const cleanup = observeVideos(initContext);
  return cleanup;
}
