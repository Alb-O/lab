import { Menu, Notice, App, Modal, MarkdownView, Plugin } from 'obsidian';
import { getVideoLinkDetails, getCurrentTimeRounded } from '../utils';
import { parseTimestampToSeconds } from '../../timestamps/utils';
import { VideoWithTimestamp } from '../../video';
import { generateMarkdownLink } from 'obsidian-dev-utils/obsidian/Link';

export function addSetStartTime(menu: Menu, plugin: Plugin, video: HTMLVideoElement) {
    menu.addItem(item =>
        item
            .setIcon('log-in')
            .setTitle('Set start time')
            .onClick(() => {
                new TimestampInputModal(plugin.app, video, 'start').open(); // Pass app directly
            })
    );
}

export function addSetEndTime(menu: Menu, plugin: Plugin, video: HTMLVideoElement) {
    menu.addItem(item =>
        item
            .setIcon('log-out')
            .setTitle('Set end time')
            .onClick(() => {
                new TimestampInputModal(plugin.app, video, 'end').open(); // Pass app directly
            })
    );
}

class TimestampInputModal extends Modal {
    video: HTMLVideoElement;
    type: 'start' | 'end';
    textareaEl: HTMLTextAreaElement | null = null;

    constructor(app: App, video: HTMLVideoElement, type: 'start' | 'end') { // Accept app instance
        super(app); // Pass app to super
        this.video = video;
        this.type = type;
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
        if (this.type === 'start' && this.video.dataset.startTime && !isNaN(Number(this.video.dataset.startTime))) {
            initialValue = this.video.dataset.startTime;
        } else if (this.type === 'end' && this.video.dataset.endTime && !isNaN(Number(this.video.dataset.endTime))) {
            initialValue = this.video.dataset.endTime;
        } else {
            initialValue = currentTimestamp.toString();
        }
        const formattedCurrent = require('../../timestamps/utils').formatTimestamp(currentTimestamp);

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
        const useCurrentBtn = buttonContainer.createEl('button', { cls: 'mod-cta' });
        useCurrentBtn.textContent = `Use Current Time (${formattedCurrent})`;
        useCurrentBtn.onclick = async () => {
            if (this.textareaEl) {
                this.textareaEl.value = currentTimestamp.toString();
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
        if (!rawInput) {
            new Notice('Timestamp cannot be empty.');
            return;
        }

        const parsedSeconds = parseTimestampToSeconds(rawInput);
        if (parsedSeconds === null) {
            new Notice('Invalid timestamp format. Use seconds (e.g., 65.5) or mm:ss (e.g., 1:05.5).');
            return;
        }

        const linkDetails = getVideoLinkDetails(this.app, this.video);
        if (!linkDetails) {
            new Notice('Error: Could not retrieve video details to update the source.');
            this.close();
            return;
        }

        const { targetFile, isExternalFileUrl, externalFileUrl } = linkDetails;
        let baseSrc: string;
        const currentSrc = this.video.currentSrc || this.video.src;

        if (isExternalFileUrl && externalFileUrl) {
            baseSrc = externalFileUrl.split('#')[0];
        } else if (targetFile) {
            baseSrc = this.app.vault.getResourcePath(targetFile).split('#')[0];
        } else if (currentSrc.startsWith('app://')) {
            const appUrlMatch = currentSrc.match(/^app:\/\/[^\/]+\/(.*)/);
            if (appUrlMatch && appUrlMatch[1]) {
                baseSrc = `file:///${appUrlMatch[1]}`.split('#')[0];
            } else {
                new Notice('Error: Could not parse app:// URL to determine base source.');
                this.close(); return;
            }
        } else if (currentSrc.startsWith('file:///')) {
            baseSrc = currentSrc.split('#')[0];
        } else {
            new Notice('Error: Could not determine video source for timestamping (unhandled URL scheme).');
            this.close(); return;
        }

        let startTime: number | undefined = undefined;
        let endTime: number | undefined = undefined;
        if (this.video.dataset.startTime && !isNaN(Number(this.video.dataset.startTime))) {
            startTime = Number(this.video.dataset.startTime);
        }
        if (this.video.dataset.endTime && !isNaN(Number(this.video.dataset.endTime))) {
            endTime = Number(this.video.dataset.endTime);
        }

        let fragment = "";
        if (this.type === 'start') {
            if (typeof endTime === 'number' && !isNaN(endTime)) {
                if (parsedSeconds >= endTime) {
                    new Notice('Start time cannot be after or at the end time.'); return;
                }
                fragment = `t=${parsedSeconds},${endTime}`;
            } else {
                fragment = `t=${parsedSeconds}`;
            }
            this.video.dataset.startTime = parsedSeconds.toString();
            if (typeof endTime === 'number' && !isNaN(endTime)) this.video.dataset.endTime = endTime.toString(); else delete this.video.dataset.endTime;
        } else { // type === 'end'
            if (typeof startTime === 'number' && !isNaN(startTime)) {
                if (parsedSeconds <= startTime) {
                    new Notice('End time cannot be before or at the start time.'); return;
                }
                fragment = `t=${startTime},${parsedSeconds}`;
            } else {
                fragment = `t=0,${parsedSeconds}`;
            }
            this.video.dataset.endTime = parsedSeconds.toString();
            if (typeof startTime === 'number' && !isNaN(startTime)) this.video.dataset.startTime = startTime.toString(); else delete this.video.dataset.startTime;
        }

        const newSrcForDom = `${baseSrc}#${fragment}`;
        const newSrcToStoreInEditor = (targetFile && !isExternalFileUrl)
            ? `${this.app.vault.getResourcePath(targetFile).split('#')[0]}#${fragment}`
            : newSrcForDom;

        this.video.src = newSrcForDom;

        const mdView = this.app.workspace.getActiveViewOfType(MarkdownView);
        if (mdView && mdView.editor && mdView.file) { // Ensure mdView.file exists
            const editor = mdView.editor;
            const currentFile = mdView.file; // Use currentFile for clarity
            try {
                const { extractVideosFromMarkdownView } = require('../../video');
                const allVideosInEditor: VideoWithTimestamp[] = extractVideosFromMarkdownView(mdView);
                let updatedInMarkdown = false;
                const domVideoElements = Array.from(mdView.contentEl.querySelectorAll('video'));
                const currentVideoDomIndex = domVideoElements.indexOf(this.video);

                if (currentVideoDomIndex !== -1 && currentVideoDomIndex < allVideosInEditor.length) {
                    const currentVideoInfo = allVideosInEditor[currentVideoDomIndex];
                    const loggableVideoInfo = {
                        ...currentVideoInfo,
                        file: currentVideoInfo.file ? currentVideoInfo.file.path : null
                    };
                    if (process.env.NODE_ENV !== 'production') {
                        console.log("TimestampInputModal: Matched video by index:", JSON.parse(JSON.stringify(loggableVideoInfo)));
                    }

                    if ((currentVideoInfo.type === 'wiki' || currentVideoInfo.type === 'md') && currentVideoInfo.file) {
                        const { line: startLine, col: startCol } = currentVideoInfo.position.start;
                        const { line: endLine, col: endCol } = currentVideoInfo.position.end;

                        const newFullMdLink = generateMarkdownLink({
                            app: this.app,
                            targetPathOrFile: currentVideoInfo.file,
                            sourcePathOrFile: currentFile, // Use currentFile (TFile)
                            subpath: `#${fragment}`,
                            isEmbed: currentVideoInfo.isEmbedded,
                            originalLink: currentVideoInfo.linktext,
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
                        new Notice(`Markdown embed link updated with timestamp.`);
                        updatedInMarkdown = true;

                    } else if (currentVideoInfo.type === 'html') {
                        const { line: startLineHtml, col: startChHtml } = currentVideoInfo.position.start; // Renamed to avoid conflict
                        const { line: endLineHtml, col: endChHtml } = currentVideoInfo.position.end;     // Renamed to avoid conflict
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
                                const modifiedLine = originalBlockLines[lineIdxInBlock].replace(
                                    /src=("|')[^"'#]+(#[^"']*)?("|')/i,
                                    `src="${newSrcToStoreInEditor}"`
                                );
                                const newBlockContentLines = [...originalBlockLines];
                                newBlockContentLines[lineIdxInBlock] = modifiedLine;

                                lines.splice(blockStartPosLine, blockEndPosLine - blockStartPosLine + 1, ...newBlockContentLines);
                                await this.app.vault.modify(currentFile, lines.join('\n'));
                                new Notice('HTML video tag updated with timestamp.');
                                updatedInMarkdown = true;
                            } else {
                                if (process.env.NODE_ENV !== 'production') {
                                    console.warn("TimestampInputModal: Found HTML video block but couldn't find src attribute line.", currentVideoInfo, originalBlockLines);
                                }
                            }
                        } else {
                            if (process.env.NODE_ENV !== 'production') {
                                console.warn("TimestampInputModal: Indexed HTML video info's linktext does not appear to be a video tag.", currentVideoInfo);
                            }
                        }
                    } else {
                        if (process.env.NODE_ENV !== 'production') {
                            console.warn("TimestampInputModal: Matched video by index has unknown or unsuitable type:", currentVideoInfo);
                        }
                    }
                } else {
                    if (process.env.NODE_ENV !== 'production') {
                        console.warn("TimestampInputModal: Clicked video not found by DOM index or index out of bounds.",
                            { domIndex: currentVideoDomIndex, extractedCount: allVideosInEditor.length });
                    }
                }

                if (!updatedInMarkdown) {
                    new Notice(`Timestamp set, but could not find or update matching embed/HTML block in markdown.`);
                }
            } catch (e: any) {
                if (process.env.NODE_ENV !== 'production') {
                    console.error('Failed to update embed link:', e);
                }
                new Notice('Error updating embed link in markdown: ' + e.message);
            }
        } else if (mdView && mdView.editor && !mdView.file) {
            new Notice('Timestamp set, but cannot update link in markdown: The current file is not saved.');
        }

        new Notice(`Video ${this.type} time set to ${parsedSeconds.toFixed(2)}s.`);

        this.close();
    }

    onClose() {
        this.contentEl.empty();
    }
}