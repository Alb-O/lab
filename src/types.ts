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

export interface BuildCache {
	builds: BlenderBuildInfo[];
	lastUpdated: string; // ISO string
	version: string; // Cache format version for future compatibility
}
