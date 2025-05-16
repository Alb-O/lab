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
    const query = new URLSearchParams(hash.replace(/^#+/, ""));
    const timeValue = query.get("t");
    if (!timeValue) return null;
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
    const startRaw = start ?? null;
    const endRaw = end ?? null;

    let startTime: number | null, endTime: number | null;
    if (startRaw && endRaw) {
        startTime = convertTime(startRaw);
        endTime = endRaw === "e" ? Infinity : convertTime(endRaw);
    } else if (startRaw) {
        startTime = convertTime(startRaw);
        endTime = -1;
    } else if (endRaw) {
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
export function convertTime(time: string): number | null {
    if (timePatterns.npt_sec.test(time)) {
        return parseFloat(time);
    }
    const mmssMatch = time.match(timePatterns.npt_mmss);
    if (mmssMatch) {
        return parseInt(mmssMatch[1]) * 60 + parseFloat(mmssMatch[2]);
    }
    const hhmmssMatch = time.match(timePatterns.npt_hhmmss);
    if (hhmmssMatch) {
        return (
            parseInt(hhmmssMatch[1]) * 3600 +
            parseInt(hhmmssMatch[2]) * 60 +
            parseFloat(hhmmssMatch[3])
        );
    }
    return null;
}

/**
 * Format seconds as a human-readable timestamp (mm:ss or hh:mm:ss)
 */
export function formatTimestamp(seconds: number): string {
    if (seconds < 0) return "00:00";
    const totalRoundedSeconds = Math.round(seconds);
    const hours = Math.floor(totalRoundedSeconds / 3600);
    const minutes = Math.floor((totalRoundedSeconds % 3600) / 60);
    const secs = totalRoundedSeconds % 60;

    if (hours > 0) {
        return `${hours}:${minutes.toString().padStart(2, '0')}:${secs
            .toString()
            .padStart(2, '0')}`;
    }
    return `${minutes.toString().padStart(2, '0')}:${secs
        .toString()
        .padStart(2, '0')}`;
}
