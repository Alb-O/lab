import { VideoFragmentsSettings } from "@settings";
import parse from 'parse-duration';

/**
 * Interface representing a temporal fragment in a media file
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
 * Check if a fragment represents a single fragment (not a range)
 */
export function isFragment(fragment: TempFragment): boolean {
    const isStartNum = typeof fragment.start === 'number';
    const isEndNum = typeof fragment.end === 'number';
    return isStartNum && isEndNum && (fragment.start as number) >= 0 && (fragment.end as number) < 0;
}

/**
 * Regular expressions for parsing different time formats
 */
const timePatterns = {
    main: /^([^,]*)(,([^,]+))?$/i, // More permissive pattern to allow any characters except commas
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
            startTime = parseFragmentToSeconds(startRaw);
            if (startTime === null) {
                console.warn("Unable to parse start time:", startRaw);
                return null; // Invalid start time format
            }
        }
    }

    if (endRaw) {
        if (endRaw.toLowerCase() === 'e' || endRaw.toLowerCase() === 'end') {
            endTime = Infinity;
        } else {
            endTime = parseFragmentToSeconds(endRaw);
            if (endTime === null) {
                console.warn("Unable to parse end time:", endRaw);
                return null; // Invalid end time format
            }
        }
    }

    // If only endRaw is provided, start time is 0
    if (endRaw && !startRaw) {
        startTime = 0;
    }
    // If only startRaw is provided, endTime remains -1 (single fragment)
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
 * Format seconds as a human-readable fragment (mm:ss or hh:mm:ss)
 * Prioritizes raw format if available in TempFragment.
 * Always uses HH:mm:ss as the base format, trims zero components if enabled,
 * and outputs raw seconds if that setting is enabled.
 */
export function formatFragment(
    totalSeconds: number,
    rawFormat?: string,
    settings?: VideoFragmentsSettings
): string {
    if (rawFormat) {
        // Basic validation for rawFormat to ensure it's a plausible fragment
        if (parseFragmentToSeconds(rawFormat) === totalSeconds) {
            return rawFormat;
        }
        // If rawFormat is provided but doesn't match totalSeconds,
        // it might be stale or incorrect. Fallback to formatting totalSeconds.
    }

    if (isNaN(totalSeconds) || totalSeconds < 0) return "00:00";    // Always use raw seconds if requested
    if (settings?.useRawSeconds) {
        // Always remove leading zeros for pure seconds output
        let secStr = totalSeconds % 1 === 0 ? totalSeconds.toString() : totalSeconds.toFixed(2).replace(/\.0+$/, '').replace(/(\.[0-9]*[1-9])0+$/, '$1');
        // Remove any leading zeros (e.g., '05' -> '5', '05.4' -> '5.4')
        secStr = secStr.replace(/^0+(?=\d)/, '');
        return secStr + " s";
    }

    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;

    // Determine if we need to show decimals for seconds
    const showDecimals = (rawFormat && rawFormat.includes('.')) || (seconds % 1 !== 0);

    let secStr: string;
    if (showDecimals) {
        // Up to 2 decimals, but trim trailing zeros
        secStr = seconds.toFixed(2)
            .replace(/\.0+$/, '')
            .replace(/(\.[0-9]*[1-9])0+$/, '$1');
        // Pad single-digit integer part for decimals (1.x–9.x)
        if (seconds >= 1 && seconds < 10) {
            secStr = secStr.replace(/^(\d)(?=\.)/, '0$1');
        }
    } else {
        secStr = Math.floor(seconds).toString();
    }    let result = `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${secStr.padStart(2, '0')}`;

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
    // Strip leading zeros ONLY from the first component when trimLeadingZeros is enabled
    if (settings?.trimLeadingZeros && parts.length > 0) {
        // Only modify the first (leading) component
        parts[0] = parts[0].replace(/^0+(?=\d)/, '');
    }
    result = parts.join(':');    // Always remove leading zeros if the result is a pure seconds string (only digits and optional period)
    if (/^0+\d+(\.\d+)?$/.test(result)) {
        result = result.replace(/^0+(?=\d)/, '');
    }

    // Add " s" suffix if the result is just a pure number (no colons)
    if (/^\d+(\.\d+)?$/.test(result)) {
        return result + " s";
    }

    return result;
}

/**
 * Parse a fragment string to seconds or percent.
 * Accepts 'start' as 0, 'end' as Infinity, percentage values (e.g., '50%'),
 * and uses parse-duration to parse natural language time expressions.
 * Returns number (seconds), Infinity, or { percent: number } for percentage.
 */
export function parseFragmentToSeconds(fragment: string): number | { percent: number } | null {
    if (!fragment || typeof fragment !== 'string') return null;
    const trimmed = fragment.trim().toLowerCase();
    
    // Debug logging to help troubleshoot
    console.log(`Attempting to parse time: '${trimmed}'`);
    
    // Handle special keywords
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
    
    // First check if it's raw seconds (just a number) for efficiency
    if (timePatterns.npt_sec.test(trimmed)) {
        const s = parseFloat(trimmed);
        console.log(`  Parsed as raw seconds: ${s}`);
        return s >= 0 ? s : null;
    }
    
    // Then try the time formats (hh:mm:ss and mm:ss) for backward compatibility
    // This is still needed for direct time formats like 10:30
    let match = trimmed.match(timePatterns.npt_hhmmss);
    if (match) {
        const h = parseInt(match[1], 10);
        const m = parseInt(match[2], 10);
        const s = parseFloat(match[3]);
        if (h < 0 || m < 0 || m >= 60 || s < 0 || s >= 60) return null;
        const result = h * 3600 + m * 60 + s;
        console.log(`  Parsed as hh:mm:ss: ${h}:${m}:${s} = ${result}s`);
        return result;
    }
    
    match = trimmed.match(timePatterns.npt_mmss);
    if (match) {
        const m = parseInt(match[1], 10);
        const s = parseFloat(match[2]);
        if (m < 0 || s < 0 || s >= 60) return null;
        if (m >= 60 && trimmed.includes(':') && !trimmed.match(/^\d+:\d+:\d+/)) {
            // if it has a colon, and minutes is >=60, but it's not hh:mm:ss, it's invalid
            console.log(`  Invalid mm:ss format (m>=60): ${m}:${s}`);
            return null;
        } else if (m >= 60) {
            const result = m * 60 + s;
            console.log(`  Parsed as mm:ss (m>=60): ${m}:${s} = ${result}s`);
            return result;
        }
        const result = m * 60 + s;
        console.log(`  Parsed as mm:ss: ${m}:${s} = ${result}s`);
        return result;
    }
    
    // Use parse-duration for time expressions
    try {
        // Try to parse the duration expression
        // parse-duration returns milliseconds, so convert to seconds
        const durationMs = parse(trimmed);
        
        if (durationMs !== null && durationMs !== undefined) {
            const seconds = durationMs / 1000;
            
            // Verify the result is reasonable for a video time (less than 24 hours)
            if (seconds < 0 || seconds > 86400) {
                console.log(`  Parsed value ${seconds}s is outside reasonable range for a video time`);
                return null;
            }
            
            console.log(`  Parsed with parse-duration: ${seconds}s`);
            return seconds;
        }
    } catch (err) {
        console.error("Error parsing time with parse-duration:", err);
    }
    
    console.log(`  Failed to parse time: '${trimmed}'`);
    return null;
}

/**
 * Resolves a fragment value (seconds, Infinity, or { percent }) to seconds given a duration.
 */
export function resolveFragmentPercent(value: number | { percent: number } | null, duration: number): number | null {
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
        T_START = formatFragment(start);
    }

    let T_END = "";
    if (endRaw) {
        T_END = endRaw.toLowerCase() === 'e' ? 'end' : endRaw;
    } else if (typeof end === 'object' && 'percent' in end) {
        T_END = `${end.percent}%`;
    } else if (end === Infinity) {
        T_END = "end";
    } else if (typeof end === 'number' && end >= 0) {
        T_END = formatFragment(end);
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
