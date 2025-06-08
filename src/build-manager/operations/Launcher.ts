import { BlenderBuildInfo } from '@/types';
import { BlenderPluginSettings } from '@/settings';
import { Notice, Platform } from 'obsidian';
import * as path from 'path';
import * as fs from 'fs';
import { EventEmitter } from 'events';
import {
	debug,
	info,
	warn,
	error,
	registerLoggerClass
} from '@/utils/obsidian-logger';

export class BlenderLauncher extends EventEmitter {
	private settings: BlenderPluginSettings;

	constructor(settings: BlenderPluginSettings) {
		super();
		registerLoggerClass(this, 'BlenderLauncher');
		debug(this, 'BlenderLauncher initialized with settings');
		this.settings = settings;
	}

	/**
	 * Launch a Blender build from the extracted directory
	 */
	async launchBuild(build: BlenderBuildInfo, extractPath: string): Promise<void> {
		// Look for blender launcher/executable in the extracted build directory
		const launcherPath = this.findBlenderLauncher(extractPath);
		if (!launcherPath) {
			const expectedName = Platform.isWin ? 'blender-launcher.exe' : 'blender executable';
			throw new Error(`${expectedName} not found in build directory`);
		}
		
		try {
			// Launch Blender as a detached process
			const { spawn } = require('child_process');
			
			// Create environment with explicit Windows environment variable support
			const env = this.getBlenderEnvironment();
			
			// Use detached: true and stdio: 'ignore' to make the process independent
			// Include the enhanced environment so Blender respects all custom variables
			const child = spawn(launcherPath, [], {
				detached: true,
				stdio: 'ignore',
				cwd: path.dirname(launcherPath),
				env: env  // Pass through all environment variables including explicitly retrieved ones
			});
			
			// Unreference the child process so Node.js can exit
			child.unref();
			this.emit('buildLaunched', build, launcherPath);
			
		} catch (error) {
			this.emit('launchError', build, error);
			throw error;
		}
	}

	/**
	 * Get environment variables for Blender launch, using settings-based variables
	 */
	private getBlenderEnvironment(): NodeJS.ProcessEnv {
		// Start with current process environment
		const env = { ...process.env };
		
		// Add any custom environment variables from settings
		Object.entries(this.settings.blenderEnvironmentVariables).forEach(([key, value]) => {
			if (value && value.trim() !== '') {
				env[key] = value.trim();
			}
		});
		
		return env;
	}

	/**
	 * Find blender launcher/executable in the extracted build directory
	 */
	private findBlenderLauncher(extractPath: string): string | null {
		if (!fs.existsSync(extractPath)) {
			return null;
		}

		// Search for blender launcher/executable recursively
		const searchForLauncher = (dir: string): string | null => {
			try {
				const entries = fs.readdirSync(dir, { withFileTypes: true });
				
				for (const entry of entries) {
					const fullPath = path.join(dir, entry.name);
							if (entry.isFile()) {
						// Platform-specific executable detection
						const isLauncher = Platform.isWin 
							? entry.name.toLowerCase() === 'blender-launcher.exe'
							: entry.name === 'blender' || entry.name === 'Blender';
						
						if (isLauncher) {
							return fullPath;
						}
					} else if (entry.isDirectory()) {
						const result = searchForLauncher(fullPath);
						if (result) return result;
					}
				}
			} catch (error) {
				console.warn(`Error searching directory ${dir}:`, error);
			}
			
			return null;
		};

		return searchForLauncher(extractPath);
	}
	
	/**
	 * Update settings
	 */
	updateSettings(settings: BlenderPluginSettings): void {
		this.settings = settings;
	}
}
