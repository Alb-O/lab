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
 * Defaults to hh:mm:ss if hours > 0, otherwise mm:ss with leading zeros.
 */
export function formatTimestamp(
    totalSeconds: number,
    rawFormat?: string
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

    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;

    // Determine if we need to show decimals for seconds
    // Show decimals if the original raw format had them or if seconds is not an integer
    const showDecimals = (rawFormat && rawFormat.includes('.')) || (seconds % 1 !== 0);

    let secStr: string;
    if (showDecimals) {
        // Aim for 1 decimal place, but allow up to 3 if present.
        // Avoid trailing zeros unless they were in raw input or part of .0
        if (rawFormat && rawFormat.includes('.')) {
            const decimalPart = rawFormat.split('.')[1];
            if (decimalPart) {
                secStr = seconds.toFixed(Math.min(decimalPart.length, 3));
            } else { // e.g. "10."
                secStr = seconds.toFixed(1);
            }
        } else {
            secStr = parseFloat(seconds.toFixed(1)).toString(); // toFixed(1) then remove trailing .0 if it's like 10.0
            if (secStr.endsWith(".0")) secStr = secStr.substring(0, secStr.length - 2);
        }
    } else {
        secStr = Math.floor(seconds).toString();
    }

    if (hours > 0) {
        return `${hours}:${minutes.toString().padStart(2, '0')}:${secStr.padStart(showDecimals ? (secStr.includes('.') ? secStr.indexOf('.') + 2 : 3) : 2, '0')}`;
    }
    return `${minutes.toString().padStart(2, '0')}:${secStr.padStart(showDecimals ? (secStr.includes('.') ? secStr.indexOf('.') + 2 : 3) : 2, '0')}`;
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
