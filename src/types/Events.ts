// Import the existing types
import type { BlenderBuildInfo } from '../types';

// Enhanced error types for better type safety
export interface BlenderError {
	code: string;
	message: string;
	cause?: Error;
	context?: Record<string, unknown>;
}

export interface DownloadError extends BlenderError {
	build: BlenderBuildInfo;
	url?: string;
	bytesDownloaded?: number;
}

export interface ExtractionError extends BlenderError {
	archivePath: string;
	targetPath?: string;
	progress?: number;
}

export interface LaunchError extends BlenderError {
	build: BlenderBuildInfo;
	executable?: string;
	command?: string;
}

export interface ScrapingError extends BlenderError {
	url?: string;
	step?: string;
}

// Event payload types for better type safety
export interface ScrapingStatusEvent {
	isActive: boolean;
	currentTask: string;
	progress: number;
	totalSteps?: number;
	error?: ScrapingError;
}

export interface DownloadProgressEvent {
	build: BlenderBuildInfo;
	bytesDownloaded: number;
	totalBytes: number;
	percentage: number;
	speed?: number;
}

export interface ExtractionProgressEvent {
	archivePath: string;
	percentage: number;
	currentFile?: string;
	extractedFiles?: number;
	totalFiles?: number;
	status?: 'extracting' | 'completed' | 'error';
}

// Directory item type for file system operations
export interface DirectoryItem {
	name: string;
	stats: {
		isDirectory(): boolean;
		mtime: Date;
	};
}

// Helper function to create properly typed errors
export function createBlenderError(
	code: string, 
	message: string, 
	cause?: Error, 
	context?: Record<string, unknown>
): BlenderError {
	return { code, message, cause, context };
}

export function createDownloadError(
	build: BlenderBuildInfo,
	message: string,
	url?: string,
	bytesDownloaded?: number,
	cause?: Error
): DownloadError {
	return {
		code: 'DOWNLOAD_ERROR',
		message,
		build,
		url,
		bytesDownloaded,
		cause
	};
}

export function createExtractionError(
	archivePath: string,
	message: string,
	targetPath?: string,
	progress?: number,
	cause?: Error
): ExtractionError {
	return {
		code: 'EXTRACTION_ERROR',
		message,
		archivePath,
		targetPath,
		progress,
		cause
	};
}

export function createLaunchError(
	build: BlenderBuildInfo,
	message: string,
	executable?: string,
	command?: string,
	cause?: Error
): LaunchError {
	return {
		code: 'LAUNCH_ERROR',
		message,
		build,
		executable,
		command,
		cause
	};
}

export function createScrapingError(
	message: string,
	url?: string,
	step?: string,
	cause?: Error
): ScrapingError {
	return {
		code: 'SCRAPING_ERROR',
		message,
		url,
		step,
		cause
	};
}

