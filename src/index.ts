// Main barrel exports for the Blender Build Manager plugin
export { default as BlenderBuildManagerPlugin } from './main';
export * from './types';
export * from './settings';
export * from './build-manager';
export * from './utils';

// Re-export types explicitly to avoid conflicts
export type {
	BlenderBuildInfo,
	DownloadProgress,
	ExtractionProgress,
	ScrapingStatus,
	BuildCache,
	BuildType,
	InstalledBuildMetadata,
	InstalledBuildsCache
} from './types';

export type {
	BlenderPluginSettings
} from './settings';
