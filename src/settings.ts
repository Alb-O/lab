import { App, Plugin, PluginSettingTab, Setting } from 'obsidian';

// Define an interface that represents the methods we need from the VideoTimestamps class
export interface IVideoTimestampsPlugin extends Plugin {
    settings: VideoTimestampsSettings;
    saveSettings(): Promise<void>;
    detectVideosInActiveView(): any[];
    videoDetector: {
        debugVideos(videos: any[]): void;
    };
}

export interface VideoTimestampsSettings {
    showStatusBarInfo: boolean;
    debugMode: boolean;
}

export const DEFAULT_SETTINGS: VideoTimestampsSettings = {
    showStatusBarInfo: true,
    debugMode: false
}

/**
 * Settings tab for the Video Timestamps plugin
 */
export class VideoTimestampsSettingTab extends PluginSettingTab {
    plugin: IVideoTimestampsPlugin;
    
    constructor(app: App, plugin: IVideoTimestampsPlugin) {
        super(app, plugin);
        this.plugin = plugin;
    }
    
    display(): void {
        const { containerEl } = this;
        containerEl.empty();
        
        containerEl.createEl('h2', { text: 'Video Timestamps Settings' });
        
        new Setting(containerEl)
            .setName('Show info in status bar')
            .setDesc('Display information about detected videos in the status bar')
            .addToggle(toggle => toggle
                .setValue(this.plugin.settings.showStatusBarInfo)
                .onChange(async (value) => {
                    this.plugin.settings.showStatusBarInfo = value;
                    await this.plugin.saveSettings();
                    // Update the status bar immediately
                    this.plugin.detectVideosInActiveView();
                }));
        
        new Setting(containerEl)
            .setName('Debug mode')
            .setDesc('Log detailed information about detected videos to the console')
            .addToggle(toggle => toggle
                .setValue(this.plugin.settings.debugMode)
                .onChange(async (value) => {
                    this.plugin.settings.debugMode = value;
                    await this.plugin.saveSettings();
                    if (value) {
                        // If debug mode is enabled, immediately show debug info
                        const videos = this.plugin.detectVideosInActiveView();
                        this.plugin.videoDetector.debugVideos(videos);
                    }
                }));
    }
}