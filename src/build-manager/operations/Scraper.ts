import { BlenderBuildInfo, Platform, Architecture, ScraperCache, StableFolder } from '@/types';
import { requestUrl, Platform as ObsidianPlatform } from 'obsidian';
import * as cheerio from 'cheerio';
import { valid, coerce, compare } from 'semver';
import { EventEmitter } from 'events';
import { 
	debug, 
	info, 
	warn, 
	error,
	registerLoggerClass 
} from '@/utils/obsidian-logger';

export class BlenderScraper extends EventEmitter {
	private static readonly DOWNLOAD_BASE_URL = 'https://download.blender.org/release/';
	private static readonly DAILY_BUILDS_URL = 'https://builder.blender.org/download/';
	private static readonly BFA_NC_BASE_URL = 'https://cloud.bforartists.de';
	private static readonly BFA_NC_WEBDAV_SHARE_TOKEN = 'JxCjbyt2fFcHjy4';
		private platform: Platform;
	private architecture: Architecture;
	private cache: ScraperCache;
	private minimumVersion: string;	constructor(minimumVersion: string = '3.0') {
		super();
		registerLoggerClass(this, 'BlenderScraper');
		debug(this, `Creating Blender scraper with minimum version ${minimumVersion}`);
		
		this.platform = this.getCurrentPlatform();
		this.architecture = this.getCurrentArchitecture();
		this.minimumVersion = minimumVersion;
		this.cache = { folders: {} };
		
		debug(this, `Platform detection completed: ${this.platform} ${this.architecture}`);
		
		info(this, 'Blender scraper created successfully');
	}
	
	/**
	 * Get the current platform
	 */
	private getCurrentPlatform(): Platform {
		debug(this, 'Detecting current platform');
		
		if (ObsidianPlatform.isWin) {
			debug(this, 'Platform detected: Windows');
			return Platform.Windows;
		} else if (ObsidianPlatform.isMacOS) {
			debug(this, 'Platform detected: macOS');
			return Platform.macOS;
		} else if (ObsidianPlatform.isLinux) {
			debug(this, 'Platform detected: Linux');
			return Platform.Linux;
		} else {
			warn(this, 'Unknown platform, defaulting to Windows');
			return Platform.Windows;
		}
	}

	/**
	 * Get the current architecture
	 */	private getCurrentArchitecture(): Architecture {
		debug(this, 'Detecting current architecture');
		const arch = process.arch;
		const result = arch === 'arm64' ? Architecture.arm64 : Architecture.x64;
		debug(this, `Architecture detected: ${arch} (using ${result})`);
		return result;
	}

	/**
	 * Generate regex filter for the current platform (includes all architectures)
	 */
	private getRegexFilter(): RegExp {
		if (this.platform === Platform.Windows) {
			return /blender-.+win.+(x64|arm64|64).+zip$/i;
		} else if (this.platform === Platform.macOS) {
			return /blender-.+(macOS|darwin).+dmg$/i;
		} else {
			return /blender-.+lin.+(x64|arm64|64).+tar+(?!.*sha256).*/i;
		}
	}

	/**
	 * Parse Blender version from string
	 */
	private parseBlenderVersion(versionStr: string): string | null {
		// Clean version string and extract version
		const cleanStr = versionStr.replace(/[^\d.-]/g, '');
		const match = cleanStr.match(/(\d+\.\d+(?:\.\d+)?)/);
		return match ? match[1] : null;
	}

	/**
	 * Check if version meets minimum requirement
	 */
	private meetsMinimumVersion(version: string): boolean {
		const cleanVersion = coerce(version);
		const cleanMinimum = coerce(this.minimumVersion);
		
		if (!cleanVersion || !cleanMinimum) return false;
		
		return compare(cleanVersion.version, cleanMinimum.version) >= 0;
	}

	/**
	 * Scrape stable builds from the official download page
	 */
	async scrapeStableReleases(): Promise<BlenderBuildInfo[]> {
		this.emit('status', 'Scraping stable releases...');
				try {
			const response = await requestUrl({ url: BlenderScraper.DOWNLOAD_BASE_URL });
			const $ = cheerio.load(response.text);
			const builds: BlenderBuildInfo[] = [];// Find all Blender version folders
			const versionLinks = $('a[href*="Blender"]').filter((_, el) => {
				const href = $(el).attr('href');
				return !!(href && /Blender(\d+\.\d+)/.test(href));
			});

			for (let i = 0; i < versionLinks.length; i++) {
				const link = $(versionLinks[i]);
				const href = link.attr('href');
				if (!href) continue;

				const versionMatch = href.match(/Blender(\d+\.\d+)/);
				if (!versionMatch) continue;

				const version = versionMatch[1];
				if (!this.meetsMinimumVersion(version)) continue;

				// Get date from adjacent text
				const dateText = link.parent().text();
				const dateMatch = dateText.match(/(\d{2}-\w{3}-\d{4})/);
				const modifiedDate = dateMatch ? new Date(dateMatch[1]) : new Date();

				// Check cache
				if (this.cache.folders[version]) {
					const cachedFolder = this.cache.folders[version];
					if (new Date(cachedFolder.modifiedDate) >= modifiedDate) {
						// Use cached builds
						builds.push(...cachedFolder.assets.map(asset => asset.blinfo[0]));
						continue;
					}
				}

				// Scrape builds for this version
				const versionBuilds = await this.scrapeVersionBuilds(href, version, modifiedDate);
				builds.push(...versionBuilds);

				// Update cache
				this.cache.folders[version] = {
					assets: versionBuilds.map(build => ({ link: build.link, blinfo: [build] })),
					modifiedDate: modifiedDate.toISOString()
				};
			}

			this.emit('status', `Found ${builds.length} stable builds`);
			return builds;
		} catch (error) {
			this.emit('error', `Error scraping stable builds: ${error}`);
			return [];
		}
	}	/**
	 * Scrape builds for a specific version
	 */
	private async scrapeVersionBuilds(versionPath: string, version: string, modifiedDate: Date): Promise<BlenderBuildInfo[]> {		try {
			const versionUrl = BlenderScraper.DOWNLOAD_BASE_URL + versionPath;
			const response = await requestUrl({ url: versionUrl });
			const $ = cheerio.load(response.text);
			const builds: BlenderBuildInfo[] = [];
			const regex = this.getRegexFilter();

			$('a').each((_, el) => {
				const link = $(el);
				const href = link.attr('href');
				if (!href || !regex.test(href)) return;

				const filename = href.split('/').pop() || href;
				const downloadUrl = href.startsWith('http') ? href : versionUrl + href;
				
				// Extract build hash from filename
				const hashMatch = filename.match(/[a-f0-9]{12}/i);
				const buildHash = hashMatch ? hashMatch[0] : null;

				// Parse commit time from page content
				const commitTime = this.parseCommitTime(link, response.text) || modifiedDate;

				builds.push({
					link: downloadUrl,
					subversion: version,
					buildHash,
					commitTime,
					branch: 'stable'
				});
			});

			return builds;		} catch (error) {
			error(this, `Failed to scrape version ${version}: ${error}`);
			return [];
		}
	}

	/**
	 * Parse commit time from HTML content
	 */
	private parseCommitTime(linkElement: cheerio.Cheerio<any>, content: string): Date | null {
		try {
			const href = linkElement.attr('href');
			if (!href) return null;

			const lines = content.split('\n');
			for (const line of lines) {
				if (line.includes(href)) {
					const timeMatch = line.match(/(\d{1,2}-\w{3}-\d{4})\s+(\d{2}:\d{2})/);
					if (timeMatch) {
						const dateStr = `${timeMatch[1]} ${timeMatch[2]} GMT`;
						return new Date(dateStr);
					}
				}
			}		} catch (error) {
			error(this, `Failed to parse commit time: ${error}`);
		}
		return null;
	}

	/**
	 * Scrape daily builds from builder.blender.org
	 */
	async scrapeAutomatedReleases(): Promise<BlenderBuildInfo[]> {
		this.emit('status', 'Scraping daily builds...');
		
		const builds: BlenderBuildInfo[] = [];
		const branches = ['daily', 'experimental', 'patch'];
		
		for (const branch of branches) {			try {
				const url = `${BlenderScraper.DAILY_BUILDS_URL}${branch}/?format=json&v=1`;
				const response = await requestUrl({ url });
				const data = JSON.parse(response.text);const platformJson = this.platform.toLowerCase() === 'macos' ? 'darwin' : this.platform.toLowerCase();
				const regexFilter = this.getRegexFilter();
				
				let filteredCount = 0;
				for (const build of data) {
					const platformMatch = build.platform === platformJson;
					const regexMatch = regexFilter.test(build.file_name);
					
					// Remove architecture filtering - let the view handle it
					if (platformMatch && regexMatch) {
						filteredCount++;

						const commitTime = new Date(build.file_mtime * 1000);
						const version = this.parseBlenderVersion(build.version) || build.version;
						
						let buildVar = '';
						if (build.patch && branch !== 'daily') {
							buildVar = build.patch;
						}
						if (build.release_cycle && branch === 'daily') {
							buildVar = build.release_cycle;
						}
						if (build.branch && branch === 'experimental') {
							buildVar = build.branch;
						}
						// Always include architecture info for later filtering in the view
						if (build.architecture) {
							buildVar += ` | ${build.architecture}`;
						}

						const subversion = buildVar ? `${version}-${buildVar}` : version;

						builds.push({
							link: build.url,
							subversion,
							buildHash: build.hash,
							commitTime,
							branch
						});
					}
				}			} catch (error) {
				error(this, `Failed to scrape ${branch} builds: ${error}`);
			}}

		this.emit('status', `Found ${builds.length} automated builds`);
		return builds;
	}

	/**
	 * Get all builds (stable + automated)
	 */
	async getAllBuilds(): Promise<BlenderBuildInfo[]> {
		const buildPromises: Promise<BlenderBuildInfo[]>[] = [];

		buildPromises.push(this.scrapeStableReleases());
		buildPromises.push(this.scrapeAutomatedReleases());

		try {
			const results = await Promise.all(buildPromises);
			const allBuilds = results.flat();
			
			// Sort by commit time (newest first)
			allBuilds.sort((a, b) => b.commitTime.getTime() - a.commitTime.getTime());
			
			this.emit('status', `Scraping complete. Found ${allBuilds.length} total builds`);
			return allBuilds;
		} catch (error) {
			this.emit('error', `Error getting all builds: ${error}`);
			return [];
		}
	}

	/**
	 * Check for new builds compared to cache
	 */
	async checkForNewBuilds(lastCheck?: Date): Promise<BlenderBuildInfo[]> {
		const builds = await this.getAllBuilds();
		
		if (!lastCheck) {
			return builds;
		}

		return builds.filter(build => build.commitTime > lastCheck);
	}

	/**
	 * Get cache data
	 */
	getCache(): ScraperCache {
		return this.cache;
	}

	/**
	 * Set cache data
	 */
	setCache(cache: ScraperCache): void {
		this.cache = cache;
	}
}
