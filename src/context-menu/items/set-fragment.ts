import { Menu, Notice, App, Modal, Plugin, setIcon } from 'obsidian';
import { getCurrentTimeRounded, setAndSaveVideoFragment } from '../utils';
import { parseFragmentToSeconds, formatFragment, TempFragment, parseTempFrag } from '../../fragments/utils';
import { VideoFragmentsSettings } from '../../settings';

export function addSetFragmentMenuItem(menu: Menu, plugin: Plugin, settings: VideoFragmentsSettings, video: HTMLVideoElement) {
    menu.addItem(item =>
        item
            .setIcon('clock')
            .setTitle('Set video fragment...')
            .onClick(() => {
                new FragmentInputModal(plugin.app, video, settings).open();
            })
    );
}

class FragmentInputModal extends Modal {
    video: HTMLVideoElement;
    settings: VideoFragmentsSettings;

    private startTimeInputEl!: HTMLTextAreaElement;
    private endTimeInputEl!: HTMLTextAreaElement;
    private useCurrentStartBtn!: HTMLButtonElement;
    private useCurrentEndBtn!: HTMLButtonElement;

    private initialStartDisplayValue: string = "";
    private initialEndDisplayValue: string = "";

    private currentTimeDisplayEl!: HTMLDivElement;
    private currentTimeUpdateInterval: number | null = null;

    private readonly videoPlayListener = () => this.startCurrentTimeUpdates();
    private readonly videoPauseListener = () => this.stopCurrentTimeUpdates();
    private readonly videoSeekingListener = () => this.updateCurrentTimeDisplay();

    constructor(app: App, video: HTMLVideoElement, settings: VideoFragmentsSettings) {
        super(app);
        this.video = video;
        this.settings = settings;
    }

    private getFragmentFromVideo(): TempFragment | null {
        let initialStart: number | { percent: number } = -1;
        let initialEnd: number | { percent: number } = -1;
        let initialStartRaw: string | undefined = undefined;
        let initialEndRaw: string | undefined = undefined;

        if (this.video.dataset.startTimeRaw) {
            initialStartRaw = this.video.dataset.startTimeRaw;
            const parsedStart = parseFragmentToSeconds(initialStartRaw);
            if (parsedStart !== null && parsedStart !== 0.001) initialStart = parsedStart;
            else initialStartRaw = undefined;
        } else if (this.video.dataset.startTime) {
            initialStartRaw = this.video.dataset.startTime;
            const parsedStart = parseFragmentToSeconds(initialStartRaw);
            if (parsedStart !== null && parsedStart !== 0.001) initialStart = parsedStart;
            else initialStartRaw = undefined;
        }

        if (this.video.dataset.endTimeRaw) {
            initialEndRaw = this.video.dataset.endTimeRaw;
            const parsedEnd = parseFragmentToSeconds(initialEndRaw);
            if (parsedEnd !== null) initialEnd = parsedEnd;
        } else if (this.video.dataset.endTime) {
            initialEndRaw = this.video.dataset.endTime;
            const parsedEnd = parseFragmentToSeconds(initialEndRaw);
            if (parsedEnd !== null) initialEnd = parsedEnd;
        }

        if ((initialStart !== -1 || initialEnd !== -1 || initialStartRaw !== undefined || initialEndRaw !== undefined)) {
            return {
                start: initialStart,
                end: initialEnd,
                startRaw: initialStartRaw,
                endRaw: initialEndRaw,
            };
        } else {
            try {
                if (this.video.currentSrc) {
                    const currentHash = new URL(this.video.currentSrc).hash;
                    return parseTempFrag(currentHash);
                }
            } catch (e) {
                if (process.env.NODE_ENV !== 'production') {
                    console.warn("FragmentInputModal: Could not parse video.currentSrc to get hash", e);
                }
            }
        }
        return null;
    }

    // Helper for percent object
    private isPercentObject(val: any): val is { percent: number } {
        return val && typeof val === 'object' && 'percent' in val && typeof val.percent === 'number';
    }

    onOpen() {
        const { contentEl, modalEl } = this;
        contentEl.empty();
        modalEl.addClass('mod-video-ts-set-fragment');

        const closeButton = contentEl.createDiv('modal-close-button');
        closeButton.onclick = () => this.close();

        const header = contentEl.createDiv('modal-header');
        header.createDiv('modal-title', el => {
            el.textContent = 'Set video fragment';
        });

        const formContent = contentEl.createDiv('modal-content');
        this.populateInputs(formContent); // Initial population

        const buttonContainer = contentEl.createDiv('modal-button-container');
        // Add current time display to the left
        this.currentTimeDisplayEl = buttonContainer.createDiv({ cls: 'video-ts-current-time-display' });
        this.updateCurrentTimeDisplay(); // Initial display

        // Add Clear Both button to the left of Save
        const clearBothBtn = buttonContainer.createEl('button', { cls: 'mod-warning', text: 'Clear both' });
        clearBothBtn.onclick = async () => {
            this.startTimeInputEl.value = "";
            this.endTimeInputEl.value = "";
            await this.handleSave();
            this.close();
        };
        const saveBtn = buttonContainer.createEl('button', { cls: 'mod-cta', text: 'Save' });
        saveBtn.onclick = async () => await this.handleSave();

        // Start updates if video is already playing
        if (!this.video.paused) {
            this.startCurrentTimeUpdates();
        }
    }

    private updateCurrentTimeDisplay() {
        if (this.currentTimeDisplayEl) {
            this.currentTimeDisplayEl.textContent = `Current time: ${formatFragment(getCurrentTimeRounded(this.video), undefined, this.settings)}`;
        }
    }

    private startCurrentTimeUpdates() {
        this.stopCurrentTimeUpdates(); // Clear existing interval if any
        if (!this.video.paused) { // Only start if video is actually playing
            this.currentTimeUpdateInterval = window.setInterval(() => {
                this.updateCurrentTimeDisplay();
            }, 100); // Update every 100ms
        }
    }

    private stopCurrentTimeUpdates() {
        if (this.currentTimeUpdateInterval !== null) {
            window.clearInterval(this.currentTimeUpdateInterval);
            this.currentTimeUpdateInterval = null;
        }
    }

    private populateInputs(container?: HTMLElement) {
        const fragment = this.getFragmentFromVideo();
        const currentVideoTime = getCurrentTimeRounded(this.video);
        const videoDuration = this.video.duration;

        if (container) { // Only create elements on first call
            // Start Time Row
            const startRow = container.createDiv({ cls: 'video-ts-modal-row' });
            
            startRow.createEl('label', { text: 'Start:', cls: 'video-ts-modal-label' });
            // Determine placeholder for start time
            let startPlaceholder = "";
            const startRawValid = fragment && fragment.startRaw && fragment.startRaw !== '0.001';
            const startNumValid = fragment && typeof fragment.start === 'number' && fragment.start >= 0 && fragment.start !== 0.001;
            const startPercentValid = fragment && this.isPercentObject(fragment.start);
            if (startRawValid || startNumValid || startPercentValid) {
                if (startRawValid) {
                    startPlaceholder = fragment!.startRaw!;
                } else if (startNumValid) {
                    startPlaceholder = formatFragment(fragment!.start as number, undefined, this.settings);
                } else if (startPercentValid && this.isPercentObject(fragment!.start)) {
                    startPlaceholder = `${fragment!.start.percent}%`;
                }
            } else {
                startPlaceholder = formatFragment(currentVideoTime, undefined, this.settings);
            }
            this.startTimeInputEl = startRow.createEl('textarea', {
                attr: { rows: 1, placeholder: startPlaceholder }
            });

            const clearStartBtn = startRow.createEl('button', { cls: 'video-ts-remove-btn' });
            setIcon(clearStartBtn, 'trash');
            clearStartBtn.onclick = () => {
                this.startTimeInputEl.value = "";
            };

            this.useCurrentStartBtn = startRow.createEl('button', { text: `Set to current time`, cls: 'video-ts-use-current-btn' });
            this.useCurrentStartBtn.onclick = () => {
                this.startTimeInputEl.value = formatFragment(getCurrentTimeRounded(this.video), undefined, this.settings);
            };

            this.startTimeInputEl.addEventListener('keydown', async (event) => {
                if (event.key === 'Enter') {
                    event.preventDefault();
                    await this.handleSave();
                }
            });

            // End Time Row
            const endRow = container.createDiv({ cls: 'video-ts-modal-row' });
            
            endRow.createEl('label', { text: 'End:', cls: 'video-ts-modal-label' });
            // Determine placeholder for end time
            let endPlaceholder = "";
            const endRawValid = fragment && fragment.endRaw && fragment.endRaw !== '0.001';
            const endNumValid = fragment && typeof fragment.end === 'number' && fragment.end >= 0 && fragment.end !== Infinity && fragment.end !== 0.001;
            const endPercentValid = fragment && this.isPercentObject(fragment.end);
            if (endRawValid || endNumValid || endPercentValid) {
                if (fragment && (fragment.end === videoDuration || (fragment.endRaw && fragment.endRaw.toLowerCase() === 'end'))) {
                    endPlaceholder = 'end';
                } else if (endRawValid) {
                    endPlaceholder = fragment!.endRaw!;
                } else if (endNumValid) {
                    endPlaceholder = formatFragment(fragment!.end as number, undefined, this.settings);
                } else if (endPercentValid && this.isPercentObject(fragment!.end)) {
                    endPlaceholder = `${fragment!.end.percent}%`;
                }
            } else {
                endPlaceholder = formatFragment(currentVideoTime, undefined, this.settings);
            }
            this.endTimeInputEl = endRow.createEl('textarea', {
                attr: { rows: 1, placeholder: endPlaceholder }
            });

            const clearEndBtn = endRow.createEl('button', { cls: 'video-ts-remove-btn' });
            setIcon(clearEndBtn, 'trash');
            clearEndBtn.onclick = () => {
                this.endTimeInputEl.value = "";
            };

            this.useCurrentEndBtn = endRow.createEl('button', { text: `Set to current time`, cls: 'video-ts-use-current-btn' });
            this.useCurrentEndBtn.onclick = () => {
                this.endTimeInputEl.value = formatFragment(getCurrentTimeRounded(this.video), undefined, this.settings);
            };
            
            this.endTimeInputEl.addEventListener('keydown', async (event) => {
                if (event.key === 'Enter') {
                    event.preventDefault();
                    await this.handleSave();
                }
            });
        }
        
        // Populate start time
        if (fragment && fragment.startRaw && fragment.startRaw !== '0.001') {
            const raw = fragment.startRaw;
            // Check if raw is a special format (contains '%', ':', or is 'start'/'end'/'e')
            const isSpecialRawFormat = raw.includes('%') || raw.includes(':') || ['start', 'end', 'e'].includes(raw.toLowerCase());
            if (isSpecialRawFormat) {
                this.initialStartDisplayValue = raw;
            } else if (typeof fragment.start === 'number' && fragment.start >= 0 && fragment.start !== 0.001) {
                // If raw is a plain number string, format the parsed numeric value for consistent precision
                this.initialStartDisplayValue = formatFragment(fragment.start, undefined, this.settings);
            } else {
                // Fallback to raw if fragment.start is not a valid number (e.g. if raw was numeric but start became {percent} somehow, less likely)
                this.initialStartDisplayValue = raw;
            }
        } else if (fragment && typeof fragment.start === 'number' && fragment.start >= 0 && fragment.start !== 0.001) {
            // No raw, or raw was placeholder/invalid. Format numeric start.
            this.initialStartDisplayValue = formatFragment(fragment.start, undefined, this.settings);
        } else if (fragment && this.isPercentObject(fragment.start)) {
            // No raw, or raw was placeholder/invalid. Start is percent object.
            this.initialStartDisplayValue = `${fragment.start.percent}%`;
        } else {
            this.initialStartDisplayValue = "";
        }
        this.startTimeInputEl.value = this.initialStartDisplayValue;

        // Populate end time
        if (fragment && (fragment.end === this.video.duration || (fragment.endRaw && fragment.endRaw.toLowerCase() === 'end'))) {
            this.initialEndDisplayValue = 'end';
        } else if (fragment && fragment.endRaw) {
            const raw = fragment.endRaw;
            const isSpecialRawFormat = raw.includes('%') || raw.includes(':') || ['start', 'end', 'e'].includes(raw.toLowerCase());
            if (isSpecialRawFormat) {
                this.initialEndDisplayValue = raw;
            } else if (typeof fragment.end === 'number' && fragment.end >= 0 && fragment.end !== Infinity) {
                // If raw is a plain number string, format the parsed numeric value for consistent precision
                this.initialEndDisplayValue = formatFragment(fragment.end, undefined, this.settings);
            } else {
                // Fallback to raw
                this.initialEndDisplayValue = raw;
            }
        } else if (fragment && typeof fragment.end === 'number' && fragment.end >= 0 && fragment.end !== Infinity) {
            // No raw, or raw was placeholder/invalid. Format numeric end.
            this.initialEndDisplayValue = formatFragment(fragment.end, undefined, this.settings);
        } else if (fragment && this.isPercentObject(fragment.end)) {
            // No raw, or raw was placeholder/invalid. End is percent object.
            this.initialEndDisplayValue = `${fragment.end.percent}%`;
        } else {
            this.initialEndDisplayValue = "";
        }
        this.endTimeInputEl.value = this.initialEndDisplayValue;
    }

    private async handleSave() {
        const rawStartTime = this.startTimeInputEl.value.trim();
        const rawEndTime = this.endTimeInputEl.value.trim();
        const videoDuration = this.video.duration;

        // Parse both times first
        const parsedStart = rawStartTime === "" ? null : parseFragmentToSeconds(rawStartTime);
        let parsedEnd: number | { percent: number } | null;
        if (rawEndTime.toLowerCase() === 'end') {
            parsedEnd = videoDuration;
        } else {
            parsedEnd = rawEndTime === "" ? null : parseFragmentToSeconds(rawEndTime);
        }

        // Validation: check for invalid formats
        if (rawStartTime !== "" && parsedStart === null) {
            new Notice('Invalid start time format.');
            this.populateInputs();
            return;
        }
        if (rawEndTime !== "" && parsedEnd === null) {
            new Notice('Invalid end time format.');
            this.populateInputs();
            return;
        }

        // Validation: check for logical order (only if both are set and both are numbers)
        if (
            parsedStart !== null &&
            parsedEnd !== null &&
            typeof parsedStart === 'number' &&
            typeof parsedEnd === 'number' &&
            parsedStart >= parsedEnd
        ) {
            new Notice('Start time cannot be after or equal to the end time.');
            this.populateInputs();
            return;
        }

        // Build the new fragment
        let newFragment: TempFragment | null = null;
        if (parsedStart !== null || parsedEnd !== null) {
            newFragment = {
                start: parsedStart !== null ? parsedStart : -1,
                startRaw: parsedStart !== null ? rawStartTime : undefined,
                end: parsedEnd !== null ? parsedEnd : -1,
                endRaw: parsedEnd !== null ? (rawEndTime.toLowerCase() === 'end' ? 'end' : rawEndTime) : undefined,
            };
        }
        // If both are cleared
        if (
            !newFragment ||
            ((typeof newFragment.start === 'number' && newFragment.start === -1) && !newFragment.startRaw &&
            (typeof newFragment.end === 'number' && newFragment.end === -1) && !newFragment.endRaw)
        ) {
            newFragment = null;
        }

        // If no change, just close
        const fragmentBefore = this.getFragmentFromVideo();
        const unchanged = (
            (!fragmentBefore && !newFragment) ||
            (fragmentBefore && newFragment &&
                fragmentBefore.start === newFragment.start &&
                fragmentBefore.end === newFragment.end &&
                fragmentBefore.startRaw === newFragment.startRaw &&
                fragmentBefore.endRaw === newFragment.endRaw)
        );
        if (unchanged) {
            this.close();
            return;
        }

        // Compose notice
        let noticeMessages: string[] = [];
        if (rawStartTime !== this.initialStartDisplayValue) {
            if (rawStartTime === "") noticeMessages.push('Start time cleared.');
            else noticeMessages.push(`Start time set to ${rawStartTime}.`);
        }
        if (rawEndTime !== this.initialEndDisplayValue) {
            if (rawEndTime === "") noticeMessages.push('End time cleared.');
            else noticeMessages.push(`End time set to ${rawEndTime}.`);
        }

        const success = await setAndSaveVideoFragment(this.app, this.video, this.settings, newFragment);
        if (success) {
            if (noticeMessages.length > 0) {
                new Notice(noticeMessages.join(' '));
            }
            this.close();
        } else {
            this.populateInputs();
        }
    }

    onClose() {
        this.stopCurrentTimeUpdates();
        // Remove video event listeners
        this.video.removeEventListener('play', this.videoPlayListener);
        this.video.removeEventListener('pause', this.videoPauseListener);
        this.video.removeEventListener('seeking', this.videoSeekingListener);
        this.video.removeEventListener('seeked', this.videoSeekingListener);

        this.contentEl.empty();
    }
}