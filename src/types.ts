export interface BlenderBuildInfo {
	link: string;
	subversion: string;
	buildHash: string | null;
	commitTime: Date;
	branch: string;
	customExecutable?: string;
}

export interface BlenderVersion {
	major: number;
	minor: number;
	patch: number;
	prerelease?: string;
}

export interface ScraperCache {
	folders: Record<string, StableFolder>;
	apiFileVersion?: string;
}

export interface StableFolder {
	assets: Array<{ link: string; blinfo: [BlenderBuildInfo] }>;
	modifiedDate: string;
}

export interface DownloadProgress {
	downloaded: number;
	total: number;
	percentage: number;
	speed?: number;
	status?: 'downloading' | 'completed' | 'error' | 'paused' | 'extracting';
	error?: string;
}

export interface ExtractionProgress {
	extractedFiles: number;
	totalFiles: number;
	percentage: number;
	status?: 'extracting' | 'completed' | 'error';
}

export interface BlenderPluginSettings {
	libraryFolder: string;
	scrapeStableBuilds: boolean;
	scrapeAutomatedBuilds: boolean;
	minimumBlenderVersion: string;
	checkForBuildsOnStartup: boolean;
	autoDownloadLatest: boolean;
	showNotifications: boolean;
	maxBuildsToKeep: number;
	autoExtract: boolean;
	cleanupArchives: boolean;
	// Additional settings for the UI
	refreshOnStartup: boolean;
	enableStableBuilds: boolean;
	enableDailyBuilds: boolean;
	enableExperimentalBuilds: boolean;
	preferredArchitecture: 'auto' | 'x64' | 'arm64';
	maxConcurrentDownloads: number;
	keepOldBuilds: boolean;
}

export const DEFAULT_SETTINGS: BlenderPluginSettings = {
	libraryFolder: '.blender',
	scrapeStableBuilds: true,
	scrapeAutomatedBuilds: true,
	minimumBlenderVersion: '3.0',
	checkForBuildsOnStartup: true,
	autoDownloadLatest: false,
	showNotifications: true,
	maxBuildsToKeep: 5,
	autoExtract: true,
	cleanupArchives: false,
	refreshOnStartup: true,
	enableStableBuilds: true,
	enableDailyBuilds: true,
	enableExperimentalBuilds: false,
	preferredArchitecture: 'auto',
	maxConcurrentDownloads: 2,
	keepOldBuilds: true
};

export enum Platform {
	Windows = 'Windows',
	macOS = 'macOS',
	Linux = 'Linux'
}

export enum Architecture {
	x64 = 'x64',
	arm64 = 'arm64'
}

export enum BuildType {
	Stable = 'stable',
	Daily = 'daily',
	Experimental = 'experimental',
	LTS = 'lts'
}

export interface ScrapingStatus {
	isActive: boolean;
	currentTask: string;
	progress: number;
	lastChecked?: Date;
	error?: string;
}
