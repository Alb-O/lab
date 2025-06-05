import { BlenderBuildInfo, BuildType } from './types';
import { BLENDER_LTS_VERSIONS } from './constants';
import type { FetchBlenderBuilds } from './buildManager';

/**
 * Handles all build filtering logic including architecture, search, branch, and type filtering
 */
export class BuildFilter {
	private buildManager: FetchBlenderBuilds;

	constructor(buildManager: FetchBlenderBuilds) {
		this.buildManager = buildManager;
	}

	/**
	 * Main filtering method that applies all filter criteria
	 */
   filterBuilds(
	   builds: BlenderBuildInfo[],
	   options: {
		   searchFilter?: string;
		   branch?: string;
		   buildType?: BuildType | 'all';
		   installedOnly?: boolean;
	   } = {}
   ): BlenderBuildInfo[] {
	   const {
		   searchFilter = '',
		   branch = 'all',
		   buildType = 'all',
		   installedOnly = false
	   } = options;

	   let filteredBuilds = builds.filter(build => {
		   // If only showing installed, filter out uninstalled builds
		   if (installedOnly) {
			   const status = this.buildManager.isBuildInstalled(build);
			   if (!status.downloaded && !status.extracted) {
				   return false;
			   }
		   }
			const matchesBranch = branch === 'all' || build.branch === branch;
			const matchesBuildType = buildType === 'all' || this.getBuildType(build) === buildType;
			
			// Apply architecture filtering based on user preference
			const matchesArchitecture = this.matchesPreferredArchitecture(build);
			
			// If no search filter, just apply branch, type, and architecture filters
			if (!searchFilter) {
				return matchesBranch && matchesBuildType && matchesArchitecture;
			}
			
			// Apply search filter with improved matching
			const matchesSearch = this.improvedFuzzyMatch(build, searchFilter);
			
			return matchesSearch && matchesBranch && matchesBuildType && matchesArchitecture;
		});

		// Special handling for LTS builds: only show the latest patch of each LTS version
		if (buildType === BuildType.LTS) {
			filteredBuilds = this.filterLatestLTSPatches(filteredBuilds);
		}

		// Sort results by relevance when there's a search filter
		if (searchFilter) {
			filteredBuilds = this.sortByRelevance(filteredBuilds, searchFilter);
		}

		return filteredBuilds;
	}

	/**
	 * Determine build type from build info
	 */
	getBuildType(build: BlenderBuildInfo): BuildType {
		const version = build.subversion.toLowerCase();
		const branch = build.branch.toLowerCase();
		
		// Check for patch builds first (branch = 'patch')
		// Use word boundary to match "-pr" as a separate component, not part of other words
		if (branch === 'patch' || /-pr(\d|$|-)/.test(version)) {
			return BuildType.Patch;
		}
		
		// Check for release candidates
		if (version.includes('candidate') || version.includes('-rc')) {
			return BuildType.ReleaseCandidate;
		}
		
		// Check for LTS releases - based on known LTS version numbers
		// LTS versions defined in constants.ts
		if (version.includes('lts') || branch.includes('lts') || this.isLTSVersion(version)) {
			return BuildType.LTS;
		}
		
		// Check for stable branch builds (but not LTS)
		if (branch === 'stable') {
			return BuildType.Stable;
		}
		
		// Check for experimental builds
		if (branch === 'experimental') {
			return BuildType.Experimental;
		}
		
		// Check for daily builds (branch = 'daily' or default)
		if (branch === 'daily') {
			return BuildType.Daily;
		}
		
		// Default to daily builds for everything else
		return BuildType.Daily;
	}

	/**
	 * Check if build matches the user's preferred architecture
	 */
	private matchesPreferredArchitecture(build: BlenderBuildInfo): boolean {
		const preferredArch = this.buildManager.getPreferredArchitecture().toLowerCase();
		const buildArch = this.extractArchitectureFromBuild(build);
		return buildArch === preferredArch;
	}

	/**
	 * Extract architecture from build info (filename or branch data)
	 */
	private extractArchitectureFromBuild(build: BlenderBuildInfo): string {
		// For automated builds (daily, experimental, patch), check if architecture info is in subversion
		if (build.branch !== 'stable') {
			// Check if architecture info is embedded in the subversion string (e.g., "4.3.0-daily | arm64")
			const archMatch = build.subversion.match(/\|\s*(x64|arm64|aarch64|x86_64|amd64)/i);
			if (archMatch) {
				return this.normalizeArchitecture(archMatch[1]);
			}
		}
		
		// For all builds, extract from filename
		const filename = build.link.split('/').pop()?.toLowerCase() || '';
		return this.extractArchitectureFromFilename(filename);
	}

	/**
	 * Extract architecture from filename
	 */
	private extractArchitectureFromFilename(filename: string): string {
		const filenameLower = filename.toLowerCase();
		
		// Look for architecture patterns in the filename
		if (filenameLower.includes('arm64')) {
			return 'arm64';
		} else if (filenameLower.includes('x64') || filenameLower.includes('amd64') || filenameLower.includes('x86_64')) {
			return 'x64';
		}
		
		// Default to x64 if no specific architecture is found (for older builds)
		return 'x64';
	}

	/**
	 * Normalize architecture names to standard format
	 */
	private normalizeArchitecture(apiArch: string): string {
		const archLower = apiArch.toLowerCase();
		switch (archLower) {
			case 'x86_64':
			case 'amd64':
			case 'x64':
				return 'x64';
			case 'arm64':
			case 'aarch64':
				return 'arm64';
			default:
				return archLower;
		}
	}

	/**
	 * Check if a version string represents an LTS version
	 */
	private isLTSVersion(version: string): boolean {
		// Extract version number from strings like "4.2.0", "3.3.1", "4.2.11-stable", etc.
		const versionMatch = version.match(/(\d+\.\d+)/);
		if (versionMatch) {
			const majorMinor = versionMatch[1];
			return (BLENDER_LTS_VERSIONS as readonly string[]).includes(majorMinor);
		}
		
		return false;
	}

	/**
	 * Filter LTS builds to show only the latest patch of each LTS version
	 */
	private filterLatestLTSPatches(builds: BlenderBuildInfo[]): BlenderBuildInfo[] {
		// Group builds by LTS version (major.minor)
		const ltsBuildsByVersion = new Map<string, BlenderBuildInfo[]>();
		
		builds.forEach(build => {
			const version = build.subversion.toLowerCase();
			
			// Extract major.minor version (e.g., "4.2" from "4.2.11-stable")
			const versionMatch = version.match(/(\d+\.\d+)/);
			if (versionMatch) {
				const majorMinor = versionMatch[1];
				
				// Only group builds that are actually from LTS versions
				if (this.isLTSVersion(majorMinor)) {
					if (!ltsBuildsByVersion.has(majorMinor)) {
						ltsBuildsByVersion.set(majorMinor, []);
					}
					ltsBuildsByVersion.get(majorMinor)!.push(build);
				}
			}
		});
		
		const latestBuilds: BlenderBuildInfo[] = [];
		
		// For each LTS version, find the latest patch
		ltsBuildsByVersion.forEach((buildsForVersion, ltsVersion) => {
			// Sort builds by patch version (descending) and commit time (newest first)
			const sortedBuilds = buildsForVersion.sort((a, b) => {
				// Extract patch numbers for comparison
				const patchA = this.extractPatchNumber(a.subversion);
				const patchB = this.extractPatchNumber(b.subversion);
				
				if (patchA !== patchB) {
					return patchB - patchA; // Higher patch number first
				}
				
				// If same patch, sort by commit time (newer first)
				return b.commitTime.getTime() - a.commitTime.getTime();
			});
			
			// Take the first (latest) build
			if (sortedBuilds.length > 0) {
				latestBuilds.push(sortedBuilds[0]);
			}
		});
		
		return latestBuilds;
	}

	/**
	 * Extract patch number from version string
	 */
	private extractPatchNumber(version: string): number {
		const patchMatch = version.match(/(\d+)\.(\d+)\.(\d+)/);
		return patchMatch ? parseInt(patchMatch[3], 10) : 0;
	}

	/**
	 * Improved fuzzy matching for build search
	 */
	private improvedFuzzyMatch(build: BlenderBuildInfo, needle: string): boolean {
		const haystack = this.getSearchableText(build);
		return this.fuzzyMatch(needle, haystack);
	}

	/**
	 * Fuzzy match algorithm with strict constraints
	 */
	private fuzzyMatch(needle: string, haystack: string): boolean {
		if (!needle || !haystack) return true;
		
		const needleLower = needle.toLowerCase();
		const haystackLower = haystack.toLowerCase();
		
		// Simple substring match first (faster for exact matches)
		if (haystackLower.includes(needleLower)) {
			return true;
		}
		
		// For longer search terms, be more strict about fuzzy matching
		if (needleLower.length > 6) {
			return false; // No fuzzy matching for long terms
		}
		
		// Fuzzy match with gap constraints
		let needleIndex = 0;
		let lastMatchIndex = -1;
		const maxGap = Math.max(2, Math.floor(needleLower.length / 2)); // Allow max gap of 2 or half the needle length
		
		for (let i = 0; i < haystackLower.length && needleIndex < needleLower.length; i++) {
			if (haystackLower[i] === needleLower[needleIndex]) {
				// Check if gap is too large
				if (lastMatchIndex >= 0 && (i - lastMatchIndex) > maxGap) {
					return false;
				}
				lastMatchIndex = i;
				needleIndex++;
			}
		}
		
		// Only consider it a match if we matched all characters
		// and the total span isn't too spread out
		if (needleIndex === needleLower.length && lastMatchIndex >= 0) {
			const span = lastMatchIndex - (haystackLower.indexOf(needleLower[0]));
			const maxSpan = needleLower.length * 3; // Allow max span of 3x the needle length
			return span <= maxSpan;
		}
		
		return false;
	}
	
	/**
	 * Highlight matching characters in text for fuzzy search - optimized for performance
	 */
	highlightMatches(needle: string, haystack: string): string {
		if (!needle || !this.fuzzyMatch(needle, haystack)) {
			return haystack;
		}
		
		// Performance optimization: detect if we're likely running with DevTools open
		// by checking if performance.now() precision is reduced (common DevTools side effect)
		const isDevToolsLikely = this.isLikelyDevToolsOpen();
		
		const needleLower = needle.toLowerCase();
		const haystackLower = haystack.toLowerCase();
		
		// For exact substring matches, highlight the entire match with span (inline element)
		if (haystackLower.includes(needleLower)) {
			const startIndex = haystackLower.indexOf(needleLower);
			const endIndex = startIndex + needleLower.length;
			
			// If DevTools likely open, use simpler highlighting
			if (isDevToolsLikely) {
				return haystack.substring(0, startIndex) + 
					   '<b>' + haystack.substring(startIndex, endIndex) + '</b>' + 
					   haystack.substring(endIndex);
			}
			
			return haystack.substring(0, startIndex) + 
				   '<span class="blender-search-highlight">' + haystack.substring(startIndex, endIndex) + '</span>' + 
				   haystack.substring(endIndex);
		}
		
		// For fuzzy matches, use different strategies based on performance context
		if (isDevToolsLikely) {
			// Simplified highlighting for DevTools performance
			return haystack; // Skip fuzzy highlighting entirely when DevTools open
		}
		
		// Full fuzzy highlighting when DevTools not detected
		let result = '';
		let needleIndex = 0;
		let inHighlight = false;
		
		for (let i = 0; i < haystack.length; i++) {
			const char = haystack[i];
			const isMatch = needleIndex < needleLower.length && 
				char.toLowerCase() === needleLower[needleIndex];
			
			if (isMatch) {
				if (!inHighlight) {
					result += '<span class="blender-search-highlight">';
					inHighlight = true;
				}
				result += char;
				needleIndex++;
			} else {
				if (inHighlight) {
					result += '</span>';
					inHighlight = false;
				}
				result += char;
			}
		}
		
		// Close any open highlight span
		if (inHighlight) {
			result += '</span>';
		}
		
		return result;
	}
	/**
	 * Detect if DevTools are likely open by checking window size discrepancies
	 */
	private isLikelyDevToolsOpen(): boolean {
		try {
			// Simple heuristic: check if window is significantly smaller than screen
			// or if there are other indicators DevTools might be docked
			const widthThreshold = window.screen.availWidth - window.outerWidth > 160;
			const heightThreshold = window.screen.availHeight - window.outerHeight > 160;
			
			// Also check if console object has been overridden (common in DevTools)
			const consoleOverridden = window.console.toString().includes('[native code]') === false;
			
			return widthThreshold || heightThreshold || consoleOverridden;
		} catch (e) {
			// If there's an error accessing these properties, assume normal mode
			return false;
		}
	}

	/**
	 * Extract searchable text from build info
	 */
	private getSearchableText(build: BlenderBuildInfo): string {
		const filename = build.link.split('/').pop() || '';
		const parts = [
			build.subversion,
			build.branch,
			build.buildHash || '',
			filename,
			build.commitTime.toLocaleDateString(),
			build.commitTime.toLocaleString()
		];
		
		return parts.join(' ').toLowerCase();
	}

	/**
	 * Sort builds by relevance to search term
	 */
	private sortByRelevance(builds: BlenderBuildInfo[], searchTerm: string): BlenderBuildInfo[] {
		return builds.sort((a, b) => {
			const scoreA = this.calculateRelevanceScore(a, searchTerm);
			const scoreB = this.calculateRelevanceScore(b, searchTerm);
			
			if (scoreA !== scoreB) {
				return scoreB - scoreA; // Higher score first
			}
			
			// If same relevance, sort by commit time (newer first)
			return b.commitTime.getTime() - a.commitTime.getTime();
		});
	}

	/**
	 * Calculate relevance score for search results
	 */
	private calculateRelevanceScore(build: BlenderBuildInfo, searchTerm: string): number {
		const searchLower = searchTerm.toLowerCase();
		let score = 0;
		
		// Exact match in subversion gets highest score
		if (build.subversion.toLowerCase().includes(searchLower)) {
			score += 100;
		}
		
		// Match in branch gets medium score
		if (build.branch.toLowerCase().includes(searchLower)) {
			score += 50;
		}
		
		// Match in build hash gets lower score
		if (build.buildHash && build.buildHash.toLowerCase().includes(searchLower)) {
			score += 25;
		}
		
		// Match in filename gets lowest score
		const filename = build.link.split('/').pop() || '';
		if (filename.toLowerCase().includes(searchLower)) {
			score += 10;
		}
		
		return score;
	}
}
