import { MarkdownView, TFile } from "obsidian";
import { splitSubpath } from "obsidian-dev-utils/obsidian/Link";
import { TempFragment, parseTempFrag } from "../timestamps/utils";

// Export video-related utilities
export { VideoDetector } from './detector';
export { setupVideoControls } from './controls';

/**
 * Represents a video with timestamp information found in a markdown document
 */
export interface VideoWithTimestamp {
    file: TFile | null;
    path: string;
    linktext: string;
    timestamp: TempFragment | null;
    isEmbedded: boolean;
    position: {
        start: { line: number; col: number };
        end: { line: number; col: number };
    };
}

/**
 * Extract video links from the current markdown view
 */
export function extractVideosFromMarkdownView(view: MarkdownView): VideoWithTimestamp[] {
    const result: VideoWithTimestamp[] = [];
    if (!view || !view.file) return result;

    const fileCache = view.app.metadataCache.getFileCache(view.file);
    if (!fileCache) return result;

    const embeds = fileCache.embeds || [];
    const links = fileCache.links || [];

    for (const embed of embeds) {
        const { link, position } = embed;
        const { linkPath: path, subpath } = splitSubpath(link);
        const file = view.app.metadataCache.getFirstLinkpathDest(path, view.file.path);
        if (file && isVideoFile(file)) {
            result.push({ file, path, linktext: link, timestamp: parseTempFrag(subpath), isEmbedded: true, position });
        }
    }
    for (const linkObj of links) {
        const { link: linktext, position } = linkObj;
        const { linkPath: path, subpath } = splitSubpath(linktext);
        const file = view.app.metadataCache.getFirstLinkpathDest(path, view.file.path);
        if (file && isVideoFile(file)) {
            result.push({ file, path, linktext, timestamp: parseTempFrag(subpath), isEmbedded: false, position });
        }
    }
    return result;
}

/**
 * Check if a file is a video file based on its extension
 */
export function isVideoFile(file: TFile): boolean {
    const videoExtensions = ['mp4', 'webm', 'ogv', 'mov', 'mkv', 'm4v'];
    return videoExtensions.includes(file.extension.toLowerCase());
}

/**
 * Observe all <video> elements in the document, including those added dynamically,
 * and invoke a callback for each one exactly once.
 * Returns a cleanup function to disconnect the observer.
 */
export function observeVideos(onVideo: (video: HTMLVideoElement) => void): () => void {
  // Initialize existing videos
  document.querySelectorAll('video').forEach(onVideo);

  // Observe for new video elements
  const observer = new MutationObserver(mutations => {
    for (const mutation of mutations) {
      if (mutation.type === 'childList') {
        for (const node of Array.from(mutation.addedNodes)) {
          if (node instanceof HTMLVideoElement) {
            onVideo(node);
          } else if (node instanceof Element) {
            node.querySelectorAll('video').forEach(onVideo);
          }
        }
      }
    }
  });
  observer.observe(document.body, { childList: true, subtree: true });

  return () => observer.disconnect();
}
