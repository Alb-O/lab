import { MarkdownView, TFile } from "obsidian";
import { splitSubpath, parseLink } from "obsidian-dev-utils/obsidian/Link";
import { TempFragment, parseTempFrag } from "../fragments/utils";

// Export video-related utilities
export { VideoDetector } from './detector';
export { setupVideoControls } from './controls';

/**
 * Represents a video with fragment information found in a markdown document
 */
export interface VideoWithFragment {
    type: 'wiki' | 'md' | 'html'; // Added
    file: TFile | null;
    path: string; // Resolved path to the TFile
    linktext: string; // Full original link text, e.g., ![[video.mp4#t=1]]
    alias?: string; // Optional alias for the link
    fragment: TempFragment | null;
    // Keep the raw format from the link so we can preserve it
    startRaw?: string;
    endRaw?: string;
    isEmbedded: boolean;
    position: {
        start: { line: number; col: number };
        end: { line: number; col: number };
    };
    originalLinkPath: string; // The path part of the link before any #subpath, e.g., "video.mp4"
    originalSubpath: string | null; // The subpath part of the link, e.g., "#t=1", or null if none
}

interface RawVideoMatch {
    type: 'wiki' | 'md' | 'html';
    // Store a simplified version of RegExpExecArray, ensuring essential properties are present.
    // The `groups` property is explicitly marked as optional to match RegExpExecArray.
    matchData: { 
        [key: string]: any; // Allows for array-like access m[0], m[1] etc.
        index: number; 
        input: string; 
        groups?: { [key: string]: string }; 
    }; 
    lineIndex: number;
}

/**
 * Extract video links from the current markdown view
 */
export function extractVideosFromMarkdownView(view: MarkdownView): VideoWithFragment[] {
    const result: VideoWithFragment[] = [];
    const activeFile = view.file;
    if (!view || !activeFile) return result;

    const editor = view.editor;
    const text = editor.getValue();
    const lines = text.split(/\r?\n/);

    const allRawMatches: RawVideoMatch[] = [];

    // Regexes
    const wikiRegex = /(!)?\[\[([^\]\|]+)(?:\|([^\]]+))?\]\]/g;
    const mdRegex = /!\[([^\]]*)\]\(([^)]+)\)|(?<!\!)\[([^\]]+)\]\(([^)]+)\)/g;
    const htmlVideoRegex = /<video[^>]*src\s*=\s*["\']([^"\'#]+)((?:#[^"\']*)?)["\'][^>]*>/gi;

    lines.forEach((lineContent, i) => {
        let m: RegExpExecArray | null;
        
        wikiRegex.lastIndex = 0;
        while ((m = wikiRegex.exec(lineContent))) {
            // Create a plain object copy of the match for stable storage
            const matchDataCopy = { ...m, groups: m.groups }; 
            allRawMatches.push({ type: 'wiki', matchData: matchDataCopy, lineIndex: i });
        }
        
        mdRegex.lastIndex = 0;
        while ((m = mdRegex.exec(lineContent))) {
            const matchDataCopy = { ...m, groups: m.groups };
            allRawMatches.push({ type: 'md', matchData: matchDataCopy, lineIndex: i });
        }

        htmlVideoRegex.lastIndex = 0;
        while ((m = htmlVideoRegex.exec(lineContent))) {
            const matchDataCopy = { ...m, groups: m.groups };
            allRawMatches.push({ type: 'html', matchData: matchDataCopy, lineIndex: i });
        }
    });

    allRawMatches.sort((a, b) => {
        if (a.lineIndex !== b.lineIndex) {
            return a.lineIndex - b.lineIndex;
        }
        return a.matchData.index - b.matchData.index;
    });

    for (const rawMatch of allRawMatches) {
        const { type, matchData, lineIndex } = rawMatch;
        const i = lineIndex;
        // Reconstruct a RegExpExecArray-like object for processing if needed, or directly use matchData
        // For this logic, direct use of matchData properties (matchData[0], matchData.index etc.) is fine.
        const m = matchData; // Use the plain object directly

        let videoEntry: VideoWithFragment | null = null;

        if (type === 'wiki') {
            const isEmbedded = !!m[1];
            const rawLinkContent = m[2]; 
            const { linkPath: parsedLinkPath, subpath: parsedSubpath } = splitSubpath(rawLinkContent);
            const file = view.app.metadataCache.getFirstLinkpathDest(parsedLinkPath, activeFile.path) || null;
            
            if (file && isVideoFile(file)) {
                const position = { start: { line: i, col: m.index }, end: { line: i, col: m.index + String(m[0]).length } };
                const fragment = parsedSubpath && parsedSubpath.toLowerCase().startsWith('#t=') ? parseTempFrag(parsedSubpath.substring(1)) : null;
                const startRaw = fragment?.startRaw;
                const endRaw = fragment?.endRaw;
                const parsedLink = parseLink(String(m[0])); // Parse the full link text for alias
                videoEntry = {
                    type: 'wiki',
                    file,
                    path: file.path,
                    linktext: String(m[0]),
                    alias: parsedLink?.alias,
                    fragment,
                    startRaw,
                    endRaw,
                    isEmbedded,
                    position,
                    originalLinkPath: parsedLinkPath,
                    originalSubpath: parsedSubpath || null
                };
            }
        } else if (type === 'md') {
            // Check if this is an embedded link (![...]) or a regular link ([...])
            const isEmbedded = m[1] !== undefined; // true for ![alt](url) format, false for [text](url)
            let linkPath = isEmbedded ? m[2] : m[4];
    
            // Split the path to extract any subpaths (fragments)
            let parsedLinkPath = linkPath;
            let parsedSubpath = null;
            if (linkPath.includes('#')) {
                const parts = linkPath.split('#');
                parsedLinkPath = parts[0];
                parsedSubpath = parts.length > 1 ? `#${parts.slice(1).join('#')}` : null;
            }
            
            // Try to resolve the file (if it's a local path) using metadata cache
            const sourcePath = activeFile.path;
            const file = view.app.metadataCache.getFirstLinkpathDest(parsedLinkPath, sourcePath);

            if (file && isVideoFile(file)) {
                // Extract fragment from subpath if it exists and starts with #t=
                let fragment: TempFragment | null = null;
                if (parsedSubpath && parsedSubpath.toLowerCase().startsWith('#t=')) {
                    fragment = parseTempFrag(parsedSubpath.substring(1)); // Remove the leading #
                }
                
                const startRaw = fragment?.startRaw;
                const endRaw = fragment?.endRaw;
                const position = {
                    start: { line: i, col: m.index },
                    end: { line: i, col: m.index + m[0].length }
                };
                
                const parsedLink = parseLink(m[0]); // Parse the full link text for alias

                videoEntry = {
                    type: 'md',
                    file, 
                    path: view.app.vault.getResourcePath(file), 
                    linktext: m[0], 
                    alias: parsedLink?.alias,
                    fragment, 
                    startRaw,
                    endRaw,
                    isEmbedded, // ![...] format is embedded, [...] format is not
                    position,
                    originalLinkPath: parsedLinkPath, 
                    originalSubpath: parsedSubpath || null
                };
            }
        } else if (type === 'html') {
            const fullHtmlTag = String(m[0]);
            const rawSrc = m[1]; 
            const subpathFragment = m[2] || undefined; 
            
            let file: TFile | null = null;
            let videoPath: string = rawSrc;
            let isLocalVideoFile = false;

            const potentialFile = view.app.metadataCache.getFirstLinkpathDest(rawSrc, activeFile.path);
            if (potentialFile && isVideoFile(potentialFile)) {
                file = potentialFile;
                videoPath = file.path;
                isLocalVideoFile = true;
            }

            const isExternalUrl = /^(https?|file):\/\//i.test(rawSrc);

            if (isLocalVideoFile || isExternalUrl) {
                const position = { start: { line: i, col: m.index }, end: { line: i, col: m.index + fullHtmlTag.length } };
                const fragment = subpathFragment ? parseTempFrag(subpathFragment.replace(/^#/, '')) : null;
                const startRaw = fragment?.startRaw;
                const endRaw = fragment?.endRaw;
                videoEntry = {
                    type: 'html', // Added
                    file, 
                    path: videoPath, 
                    linktext: fullHtmlTag, 
                    fragment, 
                    startRaw,
                    endRaw,
                    isEmbedded: true, 
                    position,
                    originalLinkPath: rawSrc, 
                    originalSubpath: subpathFragment || null
                };
            }
        }

        if (videoEntry) {
            result.push(videoEntry);
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
export function observeVideos(onVideo: (video: HTMLVideoElement) => void, getAllRelevantDocuments: () => Document[]): () => void {
    const observers: MutationObserver[] = [];

    const setupObserverForDocument = (doc: Document) => {
        // Initialize existing videos in the document
        doc.querySelectorAll('video').forEach(video => {
            const videoSrc = video.currentSrc || video.src;
            video.dataset.timestampPath = videoSrc;
            onVideo(video);
        });

        // Observe for new video elements in the document
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
        observer.observe(doc.body, { childList: true, subtree: true });
        observers.push(observer);
    };

    getAllRelevantDocuments().forEach(doc => {
        setupObserverForDocument(doc);
    });

    // Return a cleanup function to disconnect all observers
    return () => {
        observers.forEach(observer => observer.disconnect());
    };
}
