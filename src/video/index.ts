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
    path: string; // Resolved path to the TFile
    linktext: string; // Full original link text, e.g., ![[video.mp4#t=1]]
    timestamp: TempFragment | null;
    isEmbedded: boolean;
    position: {
        start: { line: number; col: number };
        end: { line: number; col: number };
    };
    originalLinkPath: string; // The path part of the link before any #subpath, e.g., "video.mp4"
    originalSubpath: string | null; // The subpath part of the link, e.g., "#t=1", or null if none
}

/**
 * Extract video links from the current markdown view
 */
export function extractVideosFromMarkdownView(view: MarkdownView): VideoWithTimestamp[] {
    const result: VideoWithTimestamp[] = [];
    const activeFile = view.file;
    if (!view || !activeFile) return result;

    const editor = view.editor;
    const text = editor.getValue();
    const lines = text.split(/\r?\n/);

    // 1) Wiki‐style embeds/links: ![[...]] and [[...]]
    const wikiRegex = /(!)?\[\[([^\]\|]+)(?:\|([^\]]+))?\]\]/g;
    lines.forEach((line, i) => {
        let m: RegExpExecArray | null;
        while ((m = wikiRegex.exec(line))) {
            const isEmbedded = !!m[1];
            const rawLinkContent = m[2]; // e.g., "video.mp4#t=1" or "video.mp4"
            const { linkPath: parsedLinkPath, subpath: parsedSubpath } = splitSubpath(rawLinkContent);
            const file = view.app.metadataCache.getFirstLinkpathDest(parsedLinkPath, activeFile.path) || null;
            if (!file || !isVideoFile(file)) continue;
            const position = {
                start: { line: i, col: m.index },
                end:   { line: i, col: m.index + m[0].length }
            };
            const timestamp = parseTempFrag(parsedSubpath);
            result.push({
                file,
                path: file.path, // Store resolved file path
                linktext: m[0],
                timestamp,
                isEmbedded,
                position,
                originalLinkPath: parsedLinkPath,
                originalSubpath: parsedSubpath ? parsedSubpath : null
            });
        }
    });

    // 2) Markdown‐style links: [alias](path#t=…)
    const mdRegex = /\[([^\]]+)\]\(([^)]+)\)/g;
    lines.forEach((line, i) => {
        let m: RegExpExecArray | null;
        while ((m = mdRegex.exec(line))) {
            const linktext = m[0];
            const url = m[2]; // e.g., "video.mp4#t=1" or "video.mp4"
            const { linkPath: parsedLinkPath, subpath: parsedSubpath } = splitSubpath(url);
            const file = view.app.metadataCache.getFirstLinkpathDest(parsedLinkPath, activeFile.path) || null;
            if (!file || !isVideoFile(file)) continue;
            const position = {
                start: { line: i, col: m.index },
                end:   { line: i, col: m.index + linktext.length }
            };
            const timestamp = parseTempFrag(parsedSubpath);
            result.push({
                file,
                path: file.path, // Store resolved file path
                linktext,
                timestamp,
                isEmbedded: false, // Markdown links are not considered "embeds" in the same way as ![[...]] for this logic
                position,
                originalLinkPath: parsedLinkPath,
                originalSubpath: parsedSubpath ? parsedSubpath : null
            });
        }
    });

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
  document.querySelectorAll('video').forEach(video => {
    const videoSrc = video.currentSrc || video.src;
    video.dataset.timestampPath = videoSrc;
    onVideo(video);
  });

  // Observe for new video elements
  const observer = new MutationObserver(mutations => {
    for (const mutation of mutations) {
      if (mutation.type === 'childList') {
        for (const node of Array.from(mutation.addedNodes)) {
          if (node instanceof HTMLVideoElement) {
            const videoSrc = node.currentSrc || node.src;
            node.dataset.timestampPath = videoSrc;
            onVideo(node);
          } else if (node instanceof Element) {
            node.querySelectorAll('video').forEach(video => {
              const videoSrc = video.currentSrc || video.src;
              video.dataset.timestampPath = videoSrc;
              onVideo(video);
            });
          }
        }
      }
    }
  });
  observer.observe(document.body, { childList: true, subtree: true });

  return () => observer.disconnect();
}
