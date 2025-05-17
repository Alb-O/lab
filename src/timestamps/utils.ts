import { VideoTimestampsSettings } from "src/settings";

/**
 * Interface representing a temporal fragment (timestamp) in a media file
 */
export interface TempFragment {
    /** Start time in seconds (-1 if not specified) */
    start: number;
    /** End time in seconds (-1 if not specified) */
    end: number;
    /** Raw string for start time, preserving original format */
    startRaw?: string;
    /** Raw string for end time, preserving original format */
    endRaw?: string;
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
    main: /^([\w:.]*)(,(?:[\w:.]+|e))?$/, // Allow 'e' for end
    npt_sec: /^\d+(?:\.\d+)?$/,
    // More flexible mm:ss and hh:mm:ss, allowing optional leading zeros and decimals on seconds
    npt_mmss: /^(\d{1,2}):([0-5]?\d(?:\.\d+)?)$/,
    npt_hhmmss: /^(\d{1,2}):([0-5]?\d):([0-5]?\d(?:\.\d+)?)$/,
};

/**
 * Parse a temporal fragment from a hash string like "t=30" or "t=1:20,1:45"
 */
export function parseTempFrag(hash: string | undefined): TempFragment | null {
    if (!hash) return null;
    const query = new URLSearchParams(hash.replace(/^#+/, ""));
    const timeValue = query.get("t");
    if (!timeValue) return null;
    const match = timeValue.match(timePatterns.main);
    if (!match) return null;

    const startRaw = match[1] === "0" ? "0" : (match[1] || undefined); // Treat "0" as a valid raw start
    const endRaw = match[2] ? match[2].substring(1) : undefined;
    return getTimeSpan(startRaw, endRaw);
}

/**
 * Convert start and end time strings to a TempFragment object
 */
function getTimeSpan(
    startRaw: string | undefined,
    endRaw: string | undefined
): TempFragment | null {
    let startTime: number | null = -1;
    let endTime: number | null = -1;

    if (startRaw) {
        startTime = convertTime(startRaw);
        if (startTime === null) return null; // Invalid start time format
    }

    if (endRaw) {
        if (endRaw.toLowerCase() === 'e') {
            endTime = Infinity;
        } else {
            endTime = convertTime(endRaw);
            if (endTime === null) return null; // Invalid end time format
        }
    }

    // If only endRaw is provided, start time is 0
    if (endRaw && !startRaw) {
        startTime = 0;
    }
    // If only startRaw is provided, endTime remains -1 (single timestamp)
    // If neither, it's not a valid fragment for our purposes (or should have been caught earlier)

    if (startTime === -1 && endTime === -1) return null;

    return {
        start: startTime !== null ? startTime : -1,
        end: endTime !== null ? endTime : -1,
        startRaw: startRaw,
        endRaw: endRaw
    };
}

/**
 * Convert a time string to seconds.
 * Supports: ss, ss.s, mm:ss, mm:ss.s, hh:mm:ss, hh:mm:ss.s
 * Allows optional leading zeros for mm and ss.
 */
export function convertTime(time: string): number | null {
    if (typeof time !== 'string' || !time.trim()) return null;
    const trimmedTime = time.trim();

    // Attempt to match hh:mm:ss format
    let match = trimmedTime.match(timePatterns.npt_hhmmss);
    if (match) {
        const h = parseInt(match[1], 10);
        const m = parseInt(match[2], 10);
        const s = parseFloat(match[3]);
        if (h < 0 || m < 0 || m >= 60 || s < 0 || s >= 60) return null;
        return h * 3600 + m * 60 + s;
    }

    // Attempt to match mm:ss format
    match = trimmedTime.match(timePatterns.npt_mmss);
    if (match) {
        const m = parseInt(match[1], 10);
        const s = parseFloat(match[2]);
        if (m < 0 || s < 0 || s >= 60) return null;
        // Disallow mm:ss if it looks like hh:mm (e.g. 65:30)
        if (m >= 60 && trimmedTime.includes(':') && !trimmedTime.match(/^\d+:\d+:\d+/)) {
            // if it has a colon, and minutes is >=60, but it's not hh:mm:ss, it's invalid
        } else if (m >= 60) { // if it's like 70:30 treat 70 as minutes
            return m * 60 + s;
        }
        return m * 60 + s;
    }

    // Attempt to match raw seconds format
    match = trimmedTime.match(timePatterns.npt_sec);
    if (match) {
        const s = parseFloat(trimmedTime);
        return s >= 0 ? s : null;
    }

    return null; // No valid format matched
}

/**
 * Format seconds as a human-readable timestamp (mm:ss or hh:mm:ss)
 * Prioritizes raw format if available in TempFragment.
 * Always uses HH:mm:ss as the base format, trims zero components if enabled,
 * and outputs raw seconds if that setting is enabled.
 */
export function formatTimestamp(
    totalSeconds: number,
    rawFormat?: string,
    settings?: VideoTimestampsSettings
): string {
    if (rawFormat) {
        // Basic validation for rawFormat to ensure it's a plausible timestamp
        if (convertTime(rawFormat) === totalSeconds) {
            return rawFormat;
        }
        // If rawFormat is provided but doesn't match totalSeconds,
        // it might be stale or incorrect. Fallback to formatting totalSeconds.
    }

    if (isNaN(totalSeconds) || totalSeconds < 0) return "00:00";

    // Always use raw seconds if requested
    if (settings?.useRawSeconds) {
        // Show decimals if present in the original seconds
        return totalSeconds % 1 === 0 ? totalSeconds.toString() : totalSeconds.toFixed(3).replace(/\.0+$/, '').replace(/(\.[0-9]*[1-9])0+$/, '$1');
    }

    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;

    // Determine if we need to show decimals for seconds
    const showDecimals = (rawFormat && rawFormat.includes('.')) || (seconds % 1 !== 0);

    let secStr: string;
    if (showDecimals) {
        // Up to 3 decimals, but trim trailing zeros
        secStr = seconds.toFixed(3)
            .replace(/\.0+$/, '')
            .replace(/(\.[0-9]*[1-9])0+$/, '$1');
        // Pad single-digit integer part for decimals (1.x–9.x)
        if (seconds >= 1 && seconds < 10) {
            secStr = secStr.replace(/^(\d)(?=\.)/, '0$1');
        }
    } else {
        secStr = Math.floor(seconds).toString();
    }

    let result = `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${secStr.padStart(2, '0')}`;

    // Split into components for trimming
    let parts = result.split(':');
    // Remove zero hours component
    if (settings?.trimZeroHours && parts.length === 3 && parseInt(parts[0], 10) === 0) {
        parts.shift();
    }
    // Remove zero minutes component when it's the leading segment
    if (settings?.trimZeroMinutes && parts.length >= 2 && parseInt(parts[0], 10) === 0) {
        parts.shift();
    }
    // Strip leading zeros from all remaining components
    if (settings?.trimLeadingZeros) {
        parts = parts.map(p => p.replace(/^0+(?=\d)/, ''));
    }
    result = parts.join(':');

    return result;
}

/**
 * Parse a timestamp string to seconds.
 * This function now primarily relies on convertTime.
 */
export function parseTimestampToSeconds(timestamp: string): number | null {
    return convertTime(timestamp);
}

/**
 * Generates a media fragment string (e.g., "t=10,20" or "t=30.5")
 * from a TempFragment object, prioritizing raw values.
 */
export function generateFragmentString(fragment: TempFragment | null): string {
    if (!fragment) return "";

    const { start, end, startRaw, endRaw } = fragment;

    let T_START = "";
    if (startRaw) {
        T_START = startRaw;
    } else if (start >= 0) {
        T_START = formatTimestamp(start); // Fallback to formatting if raw isn't there
    }

    let T_END = "";
    if (endRaw) {
        T_END = endRaw;
    } else if (end === Infinity) {
        T_END = "e";
    } else if (end >= 0) {
        T_END = formatTimestamp(end); // Fallback to formatting
    }

    if (T_START && T_END) {
        // If start is 0 and was not explicitly "0" in raw, and end exists,
        // we might simplify t=0,X to t=X, but media fragments spec says t=0,X is fine.
        // Let's be explicit for now.
        return `t=${T_START},${T_END}`;
    } else if (T_START) {
        return `t=${T_START}`;
    } else if (T_END) { // Only end time implies start=0
        return `t=0,${T_END}`;
    }
    return "";
}
