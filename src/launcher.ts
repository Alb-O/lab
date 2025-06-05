import { BlenderBuildInfo } from './types';
import { BlenderPluginSettings } from './settings';
import { Notice } from 'obsidian';
import * as path from 'path';
import * as fs from 'fs';
import { EventEmitter } from 'events';

export class BlenderLauncher extends EventEmitter {
	private settings: BlenderPluginSettings;

	constructor(settings: BlenderPluginSettings) {
		super();
		this.settings = settings;
	}
	/**
	 * Launch a Blender build from the extracted directory
	 */
	async launchBuild(build: BlenderBuildInfo, extractPath: string): Promise<void> {
		// Look for blender launcher/executable in the extracted build directory
		const launcherPath = this.findBlenderLauncher(extractPath);
		
		if (!launcherPath) {
			const expectedName = this.getExpectedExecutableName();
			throw new Error(`${expectedName} not found in build directory`);
		}
		
		try {
			// Launch Blender as a detached process
			const { spawn } = require('child_process');
			
			// Create environment with explicit Windows environment variable support
			const env = this.getBlenderEnvironment();
			
			// Configure spawn based on platform and console preference
			let child;
			if (process.platform === 'win32' && this.settings.launchWithConsole) {
				// On Windows, when launching with console, use cmd.exe to start the process
				// This ensures a console window is created, mimicking Explorer behavior
				child = spawn('cmd.exe', ['/c', 'start', '""', launcherPath], {
					detached: true,
					stdio: 'ignore',
					cwd: path.dirname(launcherPath),
					env: env,
					shell: false
				});
			} else {
				// For blender-launcher.exe or non-Windows platforms
				child = spawn(launcherPath, [], {
					detached: true,
					stdio: 'ignore',
					cwd: path.dirname(launcherPath),
					env: env
				});
			}
			
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
	 * Get the expected executable name based on platform and settings
	 */
	private getExpectedExecutableName(): string {
		if (process.platform === 'win32') {
			return this.settings.launchWithConsole ? 'blender.exe' : 'blender-launcher.exe';
		}
		return 'blender executable';
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
						const isLauncher = this.isBlenderExecutable(entry.name);
						
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
	 * Check if a filename is the correct Blender executable based on platform and settings
	 */
	private isBlenderExecutable(filename: string): boolean {
		if (process.platform === 'win32') {
			const lowerName = filename.toLowerCase();
			if (this.settings.launchWithConsole) {
				return lowerName === 'blender.exe';
			} else {
				return lowerName === 'blender-launcher.exe';
			}
		} else {
			// On non-Windows platforms, look for 'blender' or 'Blender'
			return filename === 'blender' || filename === 'Blender';
		}
	}

	/**
	 * Update settings
	 */
	updateSettings(settings: BlenderPluginSettings): void {
		this.settings = settings;
	}
}
