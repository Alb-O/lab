import { App, Plugin, PluginSettingTab, Setting } from 'obsidian';
import { VideoDetector } from '@observer';
import { FragmentManager } from '@fragments';
import { PluginEventHandler } from './plugin-event-handler';
import type { VideoWithFragment } from '@markdown';

export interface IFragmentsPlugin extends Plugin {
    settings: FragmentsSettings;
    videoDetector: VideoDetector;
    fragmentController: FragmentManager;
    pluginEventHandler: PluginEventHandler;
    detectVideosInAllDocuments(): VideoWithFragment[];
    saveSettings(): Promise<void>;
    getAllRelevantDocuments(): Document[];
}

export interface FragmentsSettings {
    loopMaxFragment: boolean;
    trimZeroHours: boolean;
    trimZeroMinutes: boolean;
    trimLeadingZeros: boolean;
    useRawSeconds: boolean;
}

export const DEFAULT_SETTINGS: FragmentsSettings = {
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
    plugin: IFragmentsPlugin;
    
    constructor(app: App, plugin: IFragmentsPlugin) {
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
                }));

        new Setting(containerEl).setName("Fragment time formatting").setHeading();

        new Setting(containerEl)
            .setName('Trim zero hours')
            .setDesc('Remove zero hours from the formatted time, e.g. 00:15:00 becomes 15:00.')
            .addToggle(toggle => toggle
                .setValue(this.plugin.settings.trimZeroHours)
                .onChange(async (value) => {
                    this.plugin.settings.trimZeroHours = value;
                    await this.plugin.saveSettings();
                }));

        new Setting(containerEl)
            .setName('Trim zero minutes')
            .setDesc('Remove zero minutes from the formatted time, e.g. 00:15 becomes 15.')
            .addToggle(toggle => toggle
                .setValue(this.plugin.settings.trimZeroMinutes)
                .onChange(async (value) => {
                    this.plugin.settings.trimZeroMinutes = value;
                    await this.plugin.saveSettings();
                }));        new Setting(containerEl)
            .setName('Trim leading zeros')
            .setDesc('Remove leading zeros from the first (largest) time component, e.g. 01:05 becomes 1:05.')
            .addToggle(toggle => toggle
                .setValue(this.plugin.settings.trimLeadingZeros)
                .onChange(async (value) => {
                    this.plugin.settings.trimLeadingZeros = value;
                    await this.plugin.saveSettings();
                }));

        new Setting(containerEl)
            .setName('Use seconds only')
            .setDesc('Purely generate seconds for fragments, no minutes or hours. Overrides all formatting options above.')
            .addToggle(toggle => toggle
                .setValue(this.plugin.settings.useRawSeconds)
                .onChange(async (value) => {
                    this.plugin.settings.useRawSeconds = value;
                    await this.plugin.saveSettings();
                }));
    }
}