/**
 * Constants used throughout the Blender builds plugin
 */

/**
 * Known Blender LTS (Long Term Support) versions
 * Updated as new LTS versions are released
 */
export const BLENDER_LTS_VERSIONS = [
	'2.83',
    '2.93',
	'3.3',
	'3.6',
	'4.2',
    // '4.5', // Uncomment when 4.5 is released
	// Future LTS versions should be added here
] as const;

/**
 * Type for LTS version strings
 */
export type BlenderLTSVersion = typeof BLENDER_LTS_VERSIONS[number];
