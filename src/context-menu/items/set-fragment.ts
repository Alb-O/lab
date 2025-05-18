import { Menu, Notice, App, Modal, Plugin, setIcon } from 'obsidian';
import { getCurrentTimeRounded, setAndSaveVideoFragment } from '../utils';
import { parseTimestampToSeconds, formatTimestamp, TempFragment, parseTempFrag } from '../../timestamps/utils';
import { VideoTimestampsSettings } from '../../settings';

export function addSetFragmentMenuItem(menu: Menu, plugin: Plugin, settings: VideoTimestampsSettings, video: HTMLVideoElement) {
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
    settings: VideoTimestampsSettings;

    private startTimeInputEl!: HTMLTextAreaElement;
    private endTimeInputEl!: HTMLTextAreaElement;
    private useCurrentStartBtn!: HTMLButtonElement;
    private useCurrentEndBtn!: HTMLButtonElement;

    private initialStartDisplayValue: string = "";
    private initialEndDisplayValue: string = "";

    constructor(app: App, video: HTMLVideoElement, settings: VideoTimestampsSettings) {
        super(app);
        this.video = video;
        this.settings = settings;
    }

    private getFragmentFromVideo(): TempFragment | null {
        let initialStart: number = -1;
        let initialEnd: number = -1;
        let initialStartRaw: string | undefined = undefined;
        let initialEndRaw: string | undefined = undefined;

        if (this.video.dataset.startTimeRaw) {
            initialStartRaw = this.video.dataset.startTimeRaw;
            const parsedStart = parseTimestampToSeconds(initialStartRaw);
            if (parsedStart !== null && parsedStart !== 0.001) initialStart = parsedStart;
            else initialStartRaw = undefined;
        } else if (this.video.dataset.startTime) {
            initialStartRaw = this.video.dataset.startTime;
            const parsedStart = parseTimestampToSeconds(initialStartRaw);
            if (parsedStart !== null && parsedStart !== 0.001) initialStart = parsedStart;
            else initialStartRaw = undefined;
        }

        if (this.video.dataset.endTimeRaw) {
            initialEndRaw = this.video.dataset.endTimeRaw;
            const parsedEnd = parseTimestampToSeconds(initialEndRaw);
            if (parsedEnd !== null) initialEnd = parsedEnd;
        } else if (this.video.dataset.endTime) {
            initialEndRaw = this.video.dataset.endTime;
            const parsedEnd = parseTimestampToSeconds(initialEndRaw);
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
                    console.warn("TimestampInputModal: Could not parse video.currentSrc to get hash", e);
                }
            }
        }
        return null;
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
        const currentTimeDisplay = buttonContainer.createDiv({ cls: 'video-ts-current-time-display' });
        currentTimeDisplay.textContent = `Current time: ${formatTimestamp(getCurrentTimeRounded(this.video), undefined, this.settings)}`;
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
            const startNumValid = fragment && fragment.start !== undefined && fragment.start >= 0 && fragment.start !== 0.001;
            if (startRawValid || startNumValid) {
                startPlaceholder = startRawValid ? fragment!.startRaw! : formatTimestamp(fragment!.start, undefined, this.settings);
            } else {
                startPlaceholder = formatTimestamp(currentVideoTime, undefined, this.settings);
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
                this.startTimeInputEl.value = formatTimestamp(getCurrentTimeRounded(this.video), undefined, this.settings);
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
            const endNumValid = fragment && fragment.end !== undefined && fragment.end >= 0 && fragment.end !== Infinity && fragment.end !== 0.001;
            if (endRawValid || endNumValid) {
                if (fragment && (fragment.end === videoDuration || (fragment.endRaw && fragment.endRaw.toLowerCase() === 'end'))) {
                    endPlaceholder = 'end';
                } else {
                    endPlaceholder = endRawValid ? fragment!.endRaw! : formatTimestamp(fragment!.end, undefined, this.settings);
                }
            } else {
                endPlaceholder = formatTimestamp(currentVideoTime, undefined, this.settings);
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
                this.endTimeInputEl.value = formatTimestamp(getCurrentTimeRounded(this.video), undefined, this.settings);
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
            this.initialStartDisplayValue = fragment.startRaw;
        } else if (fragment && fragment.start >= 0 && fragment.start !== 0.001) {
            this.initialStartDisplayValue = formatTimestamp(fragment.start, fragment.startRaw, this.settings);
        } else {
            this.initialStartDisplayValue = "";
        }
        this.startTimeInputEl.value = this.initialStartDisplayValue;

        // Populate end time
        if (fragment && (fragment.end === this.video.duration || (fragment.endRaw && fragment.endRaw.toLowerCase() === 'end'))) {
            this.initialEndDisplayValue = 'end';
        } else if (fragment && fragment.endRaw) {
            this.initialEndDisplayValue = fragment.endRaw;
        } else if (fragment && fragment.end >= 0 && fragment.end !== Infinity) {
            this.initialEndDisplayValue = formatTimestamp(fragment.end, fragment.endRaw, this.settings);
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
        const parsedStart = rawStartTime === "" ? null : parseTimestampToSeconds(rawStartTime);
        let parsedEnd: number | null;
        if (rawEndTime.toLowerCase() === 'end') {
            parsedEnd = videoDuration;
        } else {
            parsedEnd = rawEndTime === "" ? null : parseTimestampToSeconds(rawEndTime);
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

        // Validation: check for logical order (only if both are set)
        if (parsedStart !== null && parsedEnd !== null && parsedStart >= parsedEnd) {
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
        if (!newFragment || (newFragment.start === -1 && !newFragment.startRaw && newFragment.end === -1 && !newFragment.endRaw)) {
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
        this.contentEl.empty();
    }
}