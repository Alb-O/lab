import { VideoTimestampsSettings } from "src/settings";

/**
 * Interface representing a temporal fragment (timestamp) in a media file
 */
export interface TempFragment {
    /** Start time in seconds (-1 if not specified) or { percent: number } for percent */
    start: number | { percent: number };
    /** End time in seconds (-1 if not specified) or { percent: number } for percent */
    end: number | { percent: number };
    /** Raw string for start time, preserving original format */
    startRaw?: string;
    /** Raw string for end time, preserving original format */
    endRaw?: string;
}

/**
 * Check if a fragment represents a single timestamp (not a range)
 */
export function isTimestamp(fragment: TempFragment): boolean {
    const isStartNum = typeof fragment.start === 'number';
    const isEndNum = typeof fragment.end === 'number';
    return isStartNum && isEndNum && (fragment.start as number) >= 0 && (fragment.end as number) < 0;
}

/**
 * Regular expressions for parsing different time formats
 */
const timePatterns = {
    main: /^([\w:%.]*)(,(?:[\w:%.]+|e(?:nd)?))?$/i, // Allow 'e' or 'end' for end
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
    let startTime: number | { percent: number } | null = -1;
    let endTime: number | { percent: number } | null = -1;

    if (startRaw) {
        if (startRaw.toLowerCase() === 'start') {
            startTime = 0;
        } else {
            startTime = parseTimestampToSeconds(startRaw);
            if (startTime === null) return null; // Invalid start time format
        }
    }

    if (endRaw) {
        if (endRaw.toLowerCase() === 'e' || endRaw.toLowerCase() === 'end') {
            endTime = Infinity;
        } else {
            endTime = parseTimestampToSeconds(endRaw);
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
        if (parseTimestampToSeconds(rawFormat) === totalSeconds) {
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
 * Parse a timestamp string to seconds or percent.
 * Accepts 'start' as 0, 'end' as Infinity, numeric/colon formats, or percentage (e.g., '50%').
 * Returns number (seconds), Infinity, or { percent: number } for percentage.
 */
export function parseTimestampToSeconds(timestamp: string): number | { percent: number } | null {
    if (!timestamp || typeof timestamp !== 'string') return null;
    const trimmed = timestamp.trim().toLowerCase();
    if (trimmed === 'start') return 0;
    if (trimmed === 'end' || trimmed === 'e') return Infinity;
    // Percentage support
    if (trimmed.endsWith('%')) {
        const percentVal = parseFloat(trimmed.slice(0, -1));
        if (!isNaN(percentVal) && percentVal >= 0 && percentVal <= 100) {
            return { percent: percentVal };
        }
        return null;
    }
    // Fallback to numeric/colon parsing
    // Attempt to match hh:mm:ss format
    let match = trimmed.match(timePatterns.npt_hhmmss);
    if (match) {
        const h = parseInt(match[1], 10);
        const m = parseInt(match[2], 10);
        const s = parseFloat(match[3]);
        if (h < 0 || m < 0 || m >= 60 || s < 0 || s >= 60) return null;
        return h * 3600 + m * 60 + s;
    }
    // Attempt to match mm:ss format
    match = trimmed.match(timePatterns.npt_mmss);
    if (match) {
        const m = parseInt(match[1], 10);
        const s = parseFloat(match[2]);
        if (m < 0 || s < 0 || s >= 60) return null;
        if (m >= 60 && trimmed.includes(':') && !trimmed.match(/^\d+:\d+:\d+/)) {
            // if it has a colon, and minutes is >=60, but it's not hh:mm:ss, it's invalid
        } else if (m >= 60) {
            return m * 60 + s;
        }
        return m * 60 + s;
    }
    // Attempt to match raw seconds format
    match = trimmed.match(timePatterns.npt_sec);
    if (match) {
        const s = parseFloat(trimmed);
        return s >= 0 ? s : null;
    }
    return null;
}

/**
 * Resolves a timestamp value (seconds, Infinity, or { percent }) to seconds given a duration.
 */
export function resolveTimestampPercent(value: number | { percent: number } | null, duration: number): number | null {
    if (typeof value === 'number') return value;
    if (value && typeof value === 'object' && 'percent' in value) {
        return duration * (value.percent / 100);
    }
    return null;
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
        T_START = startRaw.toLowerCase() === 'start' ? 'start' : startRaw;
    } else if (typeof start === 'object' && 'percent' in start) {
        T_START = `${start.percent}%`;
    } else if (start === 0) {
        T_START = 'start';
    } else if (typeof start === 'number' && start >= 0) {
        T_START = formatTimestamp(start);
    }

    let T_END = "";
    if (endRaw) {
        T_END = endRaw.toLowerCase() === 'e' ? 'end' : endRaw;
    } else if (typeof end === 'object' && 'percent' in end) {
        T_END = `${end.percent}%`;
    } else if (end === Infinity) {
        T_END = "end";
    } else if (typeof end === 'number' && end >= 0) {
        T_END = formatTimestamp(end);
    }

    if (T_START && T_END) {
        return `t=${T_START},${T_END}`;
    } else if (T_START) {
        return `t=${T_START}`;
    } else if (T_END) {
        return `t=0,${T_END}`;
    }
    return "";
}
