import { App, Plugin, PluginSettingTab, Setting } from 'obsidian';
import { reinitializeRestrictionHandlers } from './video/restriction-handler';

// Define an interface that represents the methods we need from the VideoTimestamps class
export interface IVideoTimestampsPlugin extends Plugin {
    settings: VideoTimestampsSettings;
    saveSettings(): Promise<void>;
    detectVideosInActiveView(): any[];
}

export interface VideoTimestampsSettings {
    loopMaxTimestamp: boolean;
}

export const DEFAULT_SETTINGS: VideoTimestampsSettings = {
    loopMaxTimestamp: false
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
                
        new Setting(containerEl)
            .setName('Loop when reaching maximum timestamp')
            .setDesc('The video will automatically loop when it reaches the maximum timestamp.')
            .addToggle(toggle => toggle
                .setValue(this.plugin.settings.loopMaxTimestamp)
                .onChange(async (value) => {
                    this.plugin.settings.loopMaxTimestamp = value;
                    await this.plugin.saveSettings();
                    // Call directly from the imported function instead of through the plugin
                    reinitializeRestrictionHandlers(this.plugin.settings);
                }));
    }
}