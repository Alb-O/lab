import { MarkdownView, TFile } from "obsidian";
import { splitSubpath, testEmbed } from "obsidian-dev-utils/obsidian/Link";

/**
 * Interface representing a temporal fragment (timestamp) in a media file
 */
export interface TempFragment {
    /** Start time in seconds (-1 if not specified) */
    start: number;
    /** End time in seconds (-1 if not specified) */
    end: number;
}

/**
 * Check if a fragment represents a single timestamp (not a range)
 */
export function isTimestamp(fragment: TempFragment): boolean {
    return fragment.start >= 0 && fragment.end < 0;
}

/**
 * Regular expressions for parsing different time formats
 */
const timePatterns = {
    main: /^([\w:.]*)(,[\w:.]+)?$/,
    npt_sec: /^\d+(?:\.\d+)?$/,
    npt_mmss: /^([0-5]\d):([0-5]\d(?:\.\d+)?)$/,
    npt_hhmmss: /^(\d+):([0-5]\d):([0-5]\d(?:\.\d+)?)$/,
};

/**
 * Parse a temporal fragment from a hash string like "t=30" or "t=1:20,1:45"
 */
export function parseTempFrag(hash: string | undefined): TempFragment | null {
    if (!hash) return null;
    
    // Remove leading # and parse query params
    const query = new URLSearchParams(hash.replace(/^#+/, ""));
    const timeValue = query.get("t");
    if (!timeValue) return null;
      // Match the time pattern
    const match = timeValue.match(timePatterns.main);
    if (!match) return null;
    
    const start = match[1];
    const end = match[2] ? match[2].substring(1) : undefined;
    return getTimeSpan(start, end);
}

/**
 * Convert start and end time strings to a TempFragment object
 */
function getTimeSpan(
    start: string | undefined,
    end: string | undefined
): TempFragment | null {
    const startRaw = start ? start : null;
    const endRaw = end ?? null;

    let startTime, endTime;
    if (startRaw && endRaw) {
        startTime = convertTime(startRaw);
        // "e" is a special value meaning "end of media"
        endTime = endRaw === "e" ? Infinity : convertTime(endRaw);
    } else if (startRaw) {
        // Single timestamp
        startTime = convertTime(startRaw);
        endTime = -1;
    } else if (endRaw) {
        // End time only (start from beginning)
        startTime = 0;
        endTime = endRaw === "e" ? Infinity : convertTime(endRaw);
    } else {
        return null;
    }

    if (startTime === null || endTime === null) return null;

    return { start: startTime, end: endTime };
}

/**
 * Convert a time string to seconds
 */
function convertTime(time: string): number | null {
    // Direct seconds format (e.g., "30" or "45.5")
    if (timePatterns.npt_sec.test(time)) {
        return parseFloat(time);
    }
      // MM:SS format (e.g., "01:45")
    const mmssMatch = time.match(timePatterns.npt_mmss);
    if (mmssMatch) {
        const mm = mmssMatch[1];
        const ss = mmssMatch[2];
        return parseInt(mm) * 60 + parseFloat(ss);
    }
    
    // HH:MM:SS format (e.g., "01:30:45")
    const hhmmssMatch = time.match(timePatterns.npt_hhmmss);
    if (hhmmssMatch) {
        const hh = hhmmssMatch[1];
        const mm = hhmmssMatch[2];
        const ss = hhmmssMatch[3];
        return parseInt(hh) * 3600 + parseInt(mm) * 60 + parseFloat(ss);
    }
    
    return null;
}

/**
 * Format seconds as a human-readable timestamp (mm:ss or hh:mm:ss)
 */
export function formatTimestamp(seconds: number): string {
    if (seconds < 0) return "00:00";
    
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);
    
    if (hours > 0) {
        return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
    }
    
    return `${minutes}:${secs.toString().padStart(2, '0')}`;
}

/**
 * Represents a video with timestamp information found in a markdown document
 */
export interface VideoWithTimestamp {
    /** The file associated with the video */
    file: TFile | null;
    /** The link text (path) of the video */
    path: string;
    /** The full link text including subpath */
    linktext: string;
    /** The timestamp fragment if available */
    timestamp: TempFragment | null;
    /** Whether the video is embedded or just linked */
    isEmbedded: boolean;
    /** The link position in the document */
    position: {
        start: { line: number, col: number },
        end: { line: number, col: number }
    };
}

/**
 * Extract video links from the current markdown view
 */
export function extractVideosFromMarkdownView(view: MarkdownView): VideoWithTimestamp[] {
    const result: VideoWithTimestamp[] = [];
    
    if (!view || !view.file) return result;
    
    // Get all links from the view's cache
    const fileCache = view.app.metadataCache.getFileCache(view.file);
    if (!fileCache) return result;
    
    const embeds = fileCache.embeds || [];
    const links = fileCache.links || [];
      // Process embedded videos (![[video.mp4#t=30]])
    for (const embed of embeds) {
        const { link, position } = embed;
        // Use splitSubpath utility to properly parse the link path and subpath
        const { linkPath: path, subpath } = splitSubpath(link);
        const file = view.app.metadataCache.getFirstLinkpathDest(path, view.file.path);
        if (file && isVideoFile(file)) {
            result.push({
                file,
                path,
                linktext: link,
                timestamp: parseTempFrag(subpath),
                isEmbedded: true,
                position: position
            });
        }
    }    // Process linked videos ([[video.mp4#t=30]])
    for (const link of links) {
        const { link: linktext, position } = link;
        // Use splitSubpath utility to properly parse the link path and subpath
        const { linkPath: path, subpath } = splitSubpath(linktext);
        const file = view.app.metadataCache.getFirstLinkpathDest(path, view.file.path);
        if (file && isVideoFile(file)) {
            result.push({
                file,
                path,
                linktext,
                timestamp: parseTempFrag(subpath),
                isEmbedded: false,
                position: position
            });
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