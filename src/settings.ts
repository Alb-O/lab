import { App, Plugin, PluginSettingTab, Setting } from 'obsidian';
import { reinitializeRestrictionHandlers } from './video/restriction-handler';
import { VideoDetector, VideoWithTimestamp } from './video'; // Added imports
import { TimestampManager } from './timestamps'; // Added import
import { PluginEventHandler } from './plugin-event-handler'; // Added import

// Define an interface that represents the methods we need from the VideoTimestamps class
export interface IVideoTimestampsPlugin extends Plugin {
    settings: VideoTimestampsSettings;
    videoDetector: VideoDetector;
    timestampController: TimestampManager;
    pluginEventHandler: PluginEventHandler;
    detectVideosInAllDocuments(): VideoWithTimestamp[]; // Changed from detectVideosInActiveView
    saveSettings(): Promise<void>;
    getAllRelevantDocuments(): Document[]; // Added this
}

export interface VideoTimestampsSettings {
    loopMaxTimestamp: boolean;
    trimZeroHours: boolean;
    trimZeroMinutes: boolean;
    trimLeadingZeros: boolean;
    useRawSeconds: boolean;
}

export const DEFAULT_SETTINGS: VideoTimestampsSettings = {
    loopMaxTimestamp: false,
    trimZeroHours: true,
    trimZeroMinutes: true,
    trimLeadingZeros: false,
    useRawSeconds: false
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
                    reinitializeRestrictionHandlers(this.plugin.settings);
                }));

        new Setting(containerEl).setName("Timestamp format").setHeading();

        new Setting(containerEl)
            .setName('Trim zero hours')
            .setDesc('Remove "00:" or "0:" hours component when generating formatted timestamps.')
            .addToggle(toggle => toggle
                .setValue(this.plugin.settings.trimZeroHours)
                .onChange(async (value) => {
                    this.plugin.settings.trimZeroHours = value;
                    await this.plugin.saveSettings();
                }));

        new Setting(containerEl)
            .setName('Trim zero minutes')
            .setDesc('Remove "00:" or "0:" minutes component when generating formatted timestamps.')
            .addToggle(toggle => toggle
                .setValue(this.plugin.settings.trimZeroMinutes)
                .onChange(async (value) => {
                    this.plugin.settings.trimZeroMinutes = value;
                    await this.plugin.saveSettings();
                }));

        new Setting(containerEl)
            .setName('Trim leading zeros')
            .setDesc('Prefer single-digit time components where possible, removing leading zeros from all components.')
            .addToggle(toggle => toggle
                .setValue(this.plugin.settings.trimLeadingZeros)
                .onChange(async (value) => {
                    this.plugin.settings.trimLeadingZeros = value;
                    await this.plugin.saveSettings();
                }));

        new Setting(containerEl)
            .setName('Use raw seconds')
            .setDesc('Output pure seconds for timestamps, overriding any trimming or HH:mm:ss formatting.')
            .addToggle(toggle => toggle
                .setValue(this.plugin.settings.useRawSeconds)
                .onChange(async (value) => {
                    this.plugin.settings.useRawSeconds = value;
                    await this.plugin.saveSettings();
                }));
    }
}