import { Menu, Notice, App, Modal, MarkdownView, Plugin } from 'obsidian';
import { getVideoLinkDetails, getCurrentTimeRounded } from '../utils';
import { parseTimestampToSeconds, formatTimestamp, generateFragmentString, parseTempFrag, TempFragment } from '../../timestamps/utils';
import { VideoWithTimestamp } from '../../video';
import { generateMarkdownLink } from 'obsidian-dev-utils/obsidian/Link';
import { VideoTimestampsSettings } from '../../settings';

export function addSetStartTime(menu: Menu, plugin: Plugin, settings: VideoTimestampsSettings, video: HTMLVideoElement) {
    menu.addItem(item =>
        item
            .setIcon('log-in')
            .setTitle('Set start time')
            .onClick(() => {
                new TimestampInputModal(plugin.app, video, 'start', settings).open();
            })
    );
}

export function addSetEndTime(menu: Menu, plugin: Plugin, settings: VideoTimestampsSettings, video: HTMLVideoElement) {
    menu.addItem(item =>
        item
            .setIcon('log-out')
            .setTitle('Set end time')
            .onClick(() => {
                new TimestampInputModal(plugin.app, video, 'end', settings).open();
            })
    );
}

class TimestampInputModal extends Modal {
    video: HTMLVideoElement;
    type: 'start' | 'end';
    textareaEl: HTMLTextAreaElement | null = null;
    originalFragment: TempFragment | null = null; // Store the original fragment
    settings: VideoTimestampsSettings; // user’s chosen settings

    constructor(app: App, video: HTMLVideoElement, type: 'start' | 'end', settings: VideoTimestampsSettings) {
        super(app);
        this.video = video;
        this.type = type;
        this.settings = settings;

        let initialStart: number = -1;
        let initialEnd: number = -1;
        let initialStartRaw: string | undefined = undefined;
        let initialEndRaw: string | undefined = undefined;

        // Prioritize raw dataset attributes
        if (video.dataset.startTimeRaw) {
            initialStartRaw = video.dataset.startTimeRaw;
            const parsedStart = parseTimestampToSeconds(initialStartRaw);
            if (parsedStart !== null) initialStart = parsedStart;
        } else if (video.dataset.startTime) {
            initialStartRaw = video.dataset.startTime;
            const parsedStart = parseTimestampToSeconds(initialStartRaw);
            if (parsedStart !== null) initialStart = parsedStart;
        }

        if (video.dataset.endTimeRaw) {
            initialEndRaw = video.dataset.endTimeRaw;
            const parsedEnd = parseTimestampToSeconds(initialEndRaw);
            if (parsedEnd !== null) initialEnd = parsedEnd;
        } else if (video.dataset.endTime) {
            initialEndRaw = video.dataset.endTime;
            const parsedEnd = parseTimestampToSeconds(initialEndRaw);
            if (parsedEnd !== null) initialEnd = parsedEnd;
        }

        // Construct the fragment if we have any valid data from datasets
        if (initialStart !== -1 || initialEnd !== -1 || initialStartRaw !== undefined || initialEndRaw !== undefined) {
            this.originalFragment = {
                start: initialStart,
                end: initialEnd,
                startRaw: initialStartRaw,
                endRaw: initialEndRaw,
            };
        } else {
            // Fallback: if dataset attributes are entirely missing or empty,
            // try parsing the hash from currentSrc, acknowledging it might be a placeholder.
            try {
                if (video.currentSrc) {
                    const currentHash = new URL(video.currentSrc).hash;
                    this.originalFragment = parseTempFrag(currentHash);
                } else {
                    this.originalFragment = null;
                }
            } catch (e) {
                if (process.env.NODE_ENV !== 'production') {
                    console.warn("TimestampInputModal: Could not parse video.currentSrc to get hash", e);
                }
                this.originalFragment = null;
            }
        }
    }

    onOpen() {
        const { contentEl, modalEl } = this;
        contentEl.empty();
        modalEl.addClass('mod-video-ts-set-timestamp');

        // Close button
        const closeButton = contentEl.createDiv('modal-close-button');
        closeButton.onclick = () => this.close();

        // Header
        const header = contentEl.createDiv('modal-header');
        header.createDiv('modal-title', el => {
            el.textContent = `Set ${this.type} time`;
        });

        // Content
        const content = contentEl.createDiv('modal-content');

        const linkDetails = getVideoLinkDetails(this.app, this.video);
        if (!linkDetails) {
            new Notice('Cannot set timestamp: Video details not found.');
            this.close();
            return;
        }
        const { targetFile, isExternalFileUrl, externalFileUrl, attributesString } = linkDetails;
        const originalSrc = isExternalFileUrl ? externalFileUrl : (targetFile ? this.app.vault.getResourcePath(targetFile) : this.video.currentSrc);

        if (!originalSrc) {
            new Notice('Cannot set timestamp: Video source not found.');
            this.close();
            return;
        }

        const currentTimestamp = getCurrentTimeRounded(this.video);
        let initialValue: string;

        // Use original fragment for initial value if available and relevant
        if (this.type === 'start') {
            if (this.originalFragment && this.originalFragment.startRaw) {
                initialValue = this.originalFragment.startRaw;
            } else if (this.originalFragment && this.originalFragment.start >= 0) {
                initialValue = formatTimestamp(this.originalFragment.start, this.originalFragment.startRaw, this.settings);
            } else if (this.video.dataset.startTime) { // Fallback to dataset if needed
                initialValue = this.video.dataset.startTime;
            } else {
                initialValue = formatTimestamp(currentTimestamp, undefined, this.settings);
            }
        } else { // type === 'end'
            if (this.originalFragment && this.originalFragment.endRaw) {
                initialValue = this.originalFragment.endRaw;
            } else if (this.originalFragment && this.originalFragment.end >= 0 && this.originalFragment.end !== Infinity) {
                initialValue = formatTimestamp(this.originalFragment.end, this.originalFragment.endRaw, this.settings);
            } else if (this.video.dataset.endTime) { // Fallback to dataset
                initialValue = this.video.dataset.endTime;
            } else {
                initialValue = formatTimestamp(currentTimestamp, undefined, this.settings);
            }
        }

        const formattedCurrent = formatTimestamp(currentTimestamp, this.originalFragment?.startRaw, this.settings);

        this.textareaEl = content.createEl('textarea', {
            cls: 'video-ts-textarea',
            attr: { rows: 1 }
        });
        this.textareaEl.value = initialValue;
        this.textareaEl.placeholder = `Enter ${this.type} time (e.g., 1:23 or 83.5)`;
        this.textareaEl.addEventListener('keydown', async (event) => {
            if (event.key === 'Enter') {
                event.preventDefault();
                await this.submitTimestamp();
            }
        });

        // Button container
        const buttonContainer = contentEl.createDiv('modal-button-container');
        const saveBtn = buttonContainer.createEl('button', { cls: 'mod-cta' });
        saveBtn.textContent = 'Save';
        saveBtn.onclick = async () => {
            await this.submitTimestamp();
        };
        const clearBtn = buttonContainer.createEl('button', { cls: 'mod-warning' });
        clearBtn.textContent = 'Clear timestamp';
        clearBtn.onclick = async () => {
            await this.clearTimestamp();
        };
        const useCurrentBtn = buttonContainer.createEl('button', { cls: 'mod-cta' });
        useCurrentBtn.textContent = `Use Current Time (${formattedCurrent})`;
        useCurrentBtn.onclick = async () => {
            if (this.textareaEl) {
                this.textareaEl.value = formatTimestamp(currentTimestamp, undefined, this.settings);
                await this.submitTimestamp();
            }
        };
        const cancelBtn = buttonContainer.createEl('button', { cls: 'mod-cancel' });
        cancelBtn.textContent = 'Cancel';
        cancelBtn.onclick = () => this.close();
    }

    async submitTimestamp() {
        if (!this.textareaEl) return;
        const rawInput = this.textareaEl.value.trim();

        // We allow empty rawInput here if the intention is to clear, handled by clearTimestamp
        // However, for submit, it must not be empty if not clearing.
        if (!rawInput && this.type) { // Check this.type to ensure it's not a general call
            new Notice('Timestamp cannot be empty.');
            return;
        }

        const parsedSeconds = parseTimestampToSeconds(rawInput);
        if (parsedSeconds === null && rawInput) { // only show error if rawInput was not empty
            new Notice('Invalid timestamp format. Use seconds (e.g., 65.5) or mm:ss (e.g., 1:05.5).');
            return;
        }

        const linkDetails = getVideoLinkDetails(this.app, this.video);
        if (!linkDetails) {
            new Notice('Error: Could not retrieve video details to update the source.');
            this.close();
            return;
        }

        let baseSrc: string;
        // Use originalFragment (set in constructor) as the source of truth for current state
        // before this modal's changes.
        const currentSrcUrl = new URL(this.video.currentSrc || this.video.src);
        baseSrc = `${currentSrcUrl.protocol}//${currentSrcUrl.host}${currentSrcUrl.pathname}${currentSrcUrl.search}`;

        let newFragment: TempFragment;

        if (this.type === 'start') {
            const currentEndTime = this.originalFragment?.end;
            const currentEndRaw = this.originalFragment?.endRaw;

            if (parsedSeconds === null) { // This case should ideally be handled by clearTimestamp
                new Notice('Cannot set an empty start time. Use Clear button.'); return;
            }

            if (typeof currentEndTime === 'number' && currentEndTime >= 0 && parsedSeconds >= currentEndTime) {
                new Notice('Start time cannot be after or at the end time.'); return;
            }
            newFragment = {
                start: parsedSeconds,
                startRaw: rawInput,
                end: currentEndTime !== undefined && currentEndTime >= 0 ? currentEndTime : -1,
                endRaw: currentEndRaw
            };
            if (newFragment.startRaw) {
                this.video.dataset.startTimeRaw = newFragment.startRaw;
            } else {
                delete this.video.dataset.startTimeRaw;
            }
            this.video.dataset.startTime = newFragment.start.toString();

            if (newFragment.endRaw) {
                this.video.dataset.endTimeRaw = newFragment.endRaw;
            } else {
                delete this.video.dataset.endTimeRaw;
            }
            if (newFragment.end >= 0) {
                this.video.dataset.endTime = newFragment.end.toString();
            } else {
                delete this.video.dataset.endTime;
            }

        } else { // type === 'end'
            const currentStartTime = this.originalFragment?.start;
            const currentStartRaw = this.originalFragment?.startRaw;

            if (parsedSeconds === null) { // This case should ideally be handled by clearTimestamp
                new Notice('Cannot set an empty end time. Use Clear button.'); return;
            }

            if (typeof currentStartTime === 'number' && currentStartTime >= 0 && parsedSeconds <= currentStartTime) {
                new Notice('End time cannot be before or at the start time.'); return;
            }
            newFragment = {
                start: currentStartTime !== undefined && currentStartTime >= 0 ? currentStartTime : -1,
                startRaw: currentStartRaw,
                end: parsedSeconds,
                endRaw: rawInput
            };
            if (newFragment.startRaw) {
                this.video.dataset.startTimeRaw = newFragment.startRaw;
            } else {
                delete this.video.dataset.startTimeRaw;
            }
            this.video.dataset.startTime = newFragment.start.toString();

            if (newFragment.endRaw) {
                this.video.dataset.endTimeRaw = newFragment.endRaw;
            } else {
                delete this.video.dataset.endTimeRaw;
            }
            if (newFragment.end >= 0) {
                this.video.dataset.endTime = newFragment.end.toString();
            } else {
                delete this.video.dataset.endTime;
            }
        }

        const fragmentString = generateFragmentString(newFragment);
        const suffix = fragmentString ? `#${fragmentString}` : '';
        const newSrcForDom = `${baseSrc}${suffix}`;

        this.video.src = newSrcForDom;

        const mdView = this.app.workspace.getActiveViewOfType(MarkdownView);
        if (mdView && mdView.editor && mdView.file) { // Ensure mdView.file exists
            const currentFile = mdView.file; // Use currentFile for clarity
            try {
                await this.updateEditorLink(currentFile, fragmentString, mdView.contentEl.querySelectorAll('video'));
                new Notice(`Video ${this.type} time set to ${rawInput}.`);
            } catch (e: any) {
                if (process.env.NODE_ENV !== 'production') {
                    console.error('Failed to update embed link:', e);
                }
                new Notice('Error updating embed link in markdown: ' + e.message);
            }
        } else if (mdView && mdView.editor && !mdView.file) {
            new Notice('Timestamp set, but cannot update link in markdown: The current file is not saved.');
        }

        this.close();
    }

    async clearTimestamp() {
        const linkDetails = getVideoLinkDetails(this.app, this.video);
        if (!linkDetails) {
            new Notice('Error: Could not retrieve video details to update the source for clearing.');
            this.close();
            return;
        }

        let baseSrc: string;
        const currentSrcUrl = new URL(this.video.currentSrc || this.video.src);
        baseSrc = `${currentSrcUrl.protocol}//${currentSrcUrl.host}${currentSrcUrl.pathname}${currentSrcUrl.search}`;

        let newFragment: TempFragment | null = null;

        if (this.type === 'start') {
            if (this.originalFragment) {
                newFragment = {
                    ...this.originalFragment,
                    start: -1, // Cleared
                    startRaw: undefined
                };
                // If only start existed, and now it's cleared, the whole fragment might be empty
                if (newFragment.end < 0 && !newFragment.endRaw) newFragment = null;
            }
            delete this.video.dataset.startTimeRaw;
            delete this.video.dataset.startTime;
        } else { // type === 'end'
            if (this.originalFragment) {
                newFragment = {
                    ...this.originalFragment,
                    end: -1, // Cleared
                    endRaw: undefined
                };
                // If only end existed, and now it's cleared, the whole fragment might be empty
                // Or if start was 0 (default for only end) and end is now cleared.
                if (newFragment.start < 0 && !newFragment.startRaw) newFragment = null;
                else if (newFragment.start === 0 && !newFragment.startRaw && newFragment.end < 0) newFragment = null;

            }
            delete this.video.dataset.endTimeRaw;
            delete this.video.dataset.endTime;
        }

        // If clearing one part makes the other part a single timestamp starting at 0, remove the 0.
        if (newFragment && newFragment.start === 0 && !newFragment.startRaw && newFragment.end < 0 && !newFragment.endRaw) {
            // This case is implicitly t=0 if only end was there, then end cleared. So, no fragment.
            newFragment = null;
        }
        if (newFragment && newFragment.end === 0 && !newFragment.endRaw && newFragment.start < 0 && !newFragment.startRaw) {
            newFragment = null;
        }

        const fragmentString = newFragment ? generateFragmentString(newFragment) : '';
        const suffix = fragmentString ? `#${fragmentString}` : '';
        const newSrcForDom = `${baseSrc}${suffix}`;
        this.video.src = newSrcForDom;

        const mdView = this.app.workspace.getActiveViewOfType(MarkdownView);
        if (mdView && mdView.editor && mdView.file) {
            try {
                await this.updateEditorLink(mdView.file, fragmentString, mdView.contentEl.querySelectorAll('video'));
                new Notice(`Video ${this.type} time cleared.`);
            } catch (e: any) {
                new Notice('Error updating link in markdown after clearing: ' + e.message);
            }
        } else {
            new Notice(`Video ${this.type} time cleared. Could not update markdown link (no active file/editor).`);
        }
        this.close();
    }

    async updateEditorLink(currentFile: any, fragmentString: string, domVideoElements: NodeListOf<HTMLVideoElement>) {
        const { extractVideosFromMarkdownView } = require('../../video');
        const allVideosInEditor: VideoWithTimestamp[] = extractVideosFromMarkdownView(this.app.workspace.getActiveViewOfType(MarkdownView)!);
        const currentVideoDomIndex = Array.from(domVideoElements).indexOf(this.video);

        if (currentVideoDomIndex !== -1 && currentVideoDomIndex < allVideosInEditor.length) {
            const currentVideoInfo = allVideosInEditor[currentVideoDomIndex];
            const subpath = fragmentString ? `#${fragmentString}` : ''; // Ensure subpath is empty if fragmentString is empty

            if ((currentVideoInfo.type === 'wiki' || currentVideoInfo.type === 'md') && currentVideoInfo.file) {
                const { line: startLine, col: startCol } = currentVideoInfo.position.start;
                const { line: endLine, col: endCol } = currentVideoInfo.position.end;

                const newFullMdLink = generateMarkdownLink({
                    app: this.app,
                    targetPathOrFile: currentVideoInfo.file,
                    sourcePathOrFile: currentFile,
                    subpath: subpath,
                    isEmbed: currentVideoInfo.isEmbedded,
                    originalLink: currentVideoInfo.linktext, // This might need adjustment if alias/display text was part of old fragment
                    alias: currentVideoInfo.alias
                });

                const fileContent = await this.app.vault.read(currentFile);
                const lines = fileContent.split('\n');
                if (startLine === endLine) {
                    lines[startLine] = lines[startLine].substring(0, startCol) + newFullMdLink + lines[startLine].substring(endCol);
                } else {
                    const prefix = lines[startLine].substring(0, startCol);
                    const suffix = lines[endLine].substring(endCol);
                    const combinedLine = prefix + newFullMdLink + suffix;
                    lines.splice(startLine, endLine - startLine + 1, combinedLine);
                }
                await this.app.vault.modify(currentFile, lines.join('\n'));
            } else if (currentVideoInfo.type === 'html') {
                const { line: startLineHtml, col: startChHtml } = currentVideoInfo.position.start;
                const { line: endLineHtml, col: endChHtml } = currentVideoInfo.position.end;
                const htmlLinkText = currentVideoInfo.linktext;
                const blockStartPosLine = startLineHtml;
                const blockEndPosLine = endLineHtml;

                if (/^\s*<video[\s>]/i.test(htmlLinkText)) {
                    const fileContent = await this.app.vault.read(currentFile);
                    const lines = fileContent.split('\n');
                    const originalBlockLines = lines.slice(blockStartPosLine, blockEndPosLine + 1);
                    let lineIdxInBlock = -1;
                    for (let i = 0; i < originalBlockLines.length; i++) {
                        if (originalBlockLines[i].trim().match(/<video\s[^>]*src=/i)) {
                            lineIdxInBlock = i;
                            break;
                        }
                    }

                    if (lineIdxInBlock !== -1) {
                        // Construct the new src attribute, ensuring base path + new fragment
                        let baseVideoSrc = "";
                        const srcMatch = originalBlockLines[lineIdxInBlock].match(/src=("|')([^"'#]+)/i);
                        if (srcMatch && srcMatch[2]) {
                            baseVideoSrc = srcMatch[2];
                        }
                        const newHtmlSrcAttr = `src="${baseVideoSrc}${subpath}"`;

                        const modifiedLine = originalBlockLines[lineIdxInBlock].replace(
                            /src=("|')[^"'#]+(#[^"']*)?("|')/i,
                            newHtmlSrcAttr
                        );
                        const newBlockContentLines = [...originalBlockLines];
                        newBlockContentLines[lineIdxInBlock] = modifiedLine;

                        lines.splice(blockStartPosLine, blockEndPosLine - blockStartPosLine + 1, ...newBlockContentLines);
                        await this.app.vault.modify(currentFile, lines.join('\n'));
                    } else { /* console.warn */ }
                } else { /* console.warn */ }
            } else { /* console.warn */ }
        } else { /* console.warn */ }
    }

    onClose() {
        this.contentEl.empty();
    }
}