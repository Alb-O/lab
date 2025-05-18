import { App, Plugin, PluginSettingTab, Setting } from 'obsidian';
import { reinitializeRestrictionHandlers } from './video/restriction-handler';
import { VideoDetector, VideoWithFragment } from './video';
import { FragmentManager } from './fragments';
import { PluginEventHandler } from './plugin-event-handler';

export interface IVideoFragmentsPlugin extends Plugin {
    settings: VideoFragmentsSettings;
    videoDetector: VideoDetector;
    fragmentController: FragmentManager;
    pluginEventHandler: PluginEventHandler;
    detectVideosInAllDocuments(): VideoWithFragment[];
    saveSettings(): Promise<void>;
    getAllRelevantDocuments(): Document[];
}

export interface VideoFragmentsSettings {
    loopMaxFragment: boolean;
    trimZeroHours: boolean;
    trimZeroMinutes: boolean;
    trimLeadingZeros: boolean;
    useRawSeconds: boolean;
}

export const DEFAULT_SETTINGS: VideoFragmentsSettings = {
    loopMaxFragment: false,
    trimZeroHours: true,
    trimZeroMinutes: true,
    trimLeadingZeros: false,
    useRawSeconds: false
}

/**
 * Settings tab for the Video Fragments plugin
 */
export class VideoFragmentsSettingTab extends PluginSettingTab {
    plugin: IVideoFragmentsPlugin;
    
    constructor(app: App, plugin: IVideoFragmentsPlugin) {
        super(app, plugin);
        this.plugin = plugin;
    }
    
    display(): void {
        const { containerEl } = this;
        containerEl.empty();
                
        new Setting(containerEl)
            .setName('Loop when reaching maximum fragment')
            .setDesc('The video will automatically loop when it reaches the maximum time defined by the fragment.')
            .addToggle(toggle => toggle
                .setValue(this.plugin.settings.loopMaxFragment)
                .onChange(async (value) => {
                    this.plugin.settings.loopMaxFragment = value;
                    await this.plugin.saveSettings();
                    reinitializeRestrictionHandlers(this.plugin.settings);
                }));

        new Setting(containerEl).setName("Fragment time format").setHeading();

        new Setting(containerEl)
            .setName('Trim zero hours')
            .setDesc('Remove "00:" or "0:" hours component when generating formatted time fragments.')
            .addToggle(toggle => toggle
                .setValue(this.plugin.settings.trimZeroHours)
                .onChange(async (value) => {
                    this.plugin.settings.trimZeroHours = value;
                    await this.plugin.saveSettings();
                }));

        new Setting(containerEl)
            .setName('Trim zero minutes')
            .setDesc('Remove "00:" or "0:" minutes component when generating formatted time fragments.')
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
            .setDesc('Output pure seconds for fragments, overriding any trimming or HH:mm:ss formatting.')
            .addToggle(toggle => toggle
                .setValue(this.plugin.settings.useRawSeconds)
                .onChange(async (value) => {
                    this.plugin.settings.useRawSeconds = value;
                    await this.plugin.saveSettings();
                }));
    }
}