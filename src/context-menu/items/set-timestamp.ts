import { Menu, Notice, App, Modal, TextComponent } from 'obsidian';
import { getVideoLinkDetails, getCurrentTimeRounded } from '../utils';
import { parseTimestampToSeconds } from '../../timestamps/utils'; // Assuming this will be created

export function addSetStartTime(menu: Menu, app: App, video: HTMLVideoElement) {
  menu.addItem(item =>
    item
      .setIcon('log-in')
      .setTitle('Set start time')
      .onClick(() => {
        new TimestampInputModal(app, video, 'start').open();
      })
  );
}

export function addSetEndTime(menu: Menu, app: App, video: HTMLVideoElement) {
  menu.addItem(item =>
    item
      .setIcon('log-out')
      .setTitle('Set end time')
      .onClick(() => {
        new TimestampInputModal(app, video, 'end').open();
      })
  );
}

class TimestampInputModal extends Modal {
  video: HTMLVideoElement;
  type: 'start' | 'end';
  textareaEl: HTMLTextAreaElement | null = null;

  constructor(app: App, video: HTMLVideoElement, type: 'start' | 'end') {
    super(app);
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
      attr: { rows: 1, style: 'height: 29px;' }
    });
    this.textareaEl.value = initialValue;
    this.textareaEl.placeholder = `Enter ${this.type} time (e.g., 1:23 or 83.5)`;
    this.textareaEl.style.width = '100%';
    this.textareaEl.addEventListener('keydown', (event) => {
      if (event.key === 'Enter') {
        event.preventDefault();
        this.submitTimestamp();
      }
    });

    // Button container
    const buttonContainer = contentEl.createDiv('modal-button-container');
    const saveBtn = buttonContainer.createEl('button', { cls: 'mod-cta' });
    saveBtn.textContent = 'Save';
    saveBtn.onclick = () => {
      this.submitTimestamp();
    };
    const useCurrentBtn = buttonContainer.createEl('button', { cls: 'mod-cta' });
    useCurrentBtn.textContent = `Use Current Time (${formattedCurrent})`;
    useCurrentBtn.onclick = () => {
      if (this.textareaEl) {
        this.textareaEl.value = currentTimestamp.toString();
        this.submitTimestamp();
      }
    };
    const cancelBtn = buttonContainer.createEl('button', { cls: 'mod-cancel' });
    cancelBtn.textContent = 'Cancel';
    cancelBtn.onclick = () => this.close();
  }

  submitTimestamp() {
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

    const { targetFile, sourcePathForLink, isExternalFileUrl, externalFileUrl, attributesString } = linkDetails;
    let baseSrc: string;

    if (isExternalFileUrl && externalFileUrl) {
      baseSrc = externalFileUrl.split('#')[0];
    } else if (targetFile) {
      baseSrc = this.video.currentSrc.split('#')[0];
    } else {
      new Notice('Could not determine video source for timestamping.');
      this.close();
      return;
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
          new Notice('Start time cannot be after or at the end time.');
          return;
        }
        this.video.dataset.startTime = parsedSeconds.toString();
        this.video.dataset.endTime = endTime.toString();
        fragment = `t=${parsedSeconds},${endTime}`;
      } else {
        this.video.dataset.startTime = parsedSeconds.toString();
        delete this.video.dataset.endTime;
        fragment = `t=${parsedSeconds}`;
      }
    } else {
      if (typeof startTime === 'number' && !isNaN(startTime)) {
        if (parsedSeconds <= startTime) {
          new Notice('End time cannot be before or at the start time.');
          return;
        }
        this.video.dataset.startTime = startTime.toString();
        this.video.dataset.endTime = parsedSeconds.toString();
        fragment = `t=${startTime},${parsedSeconds}`;
      } else {
        this.video.dataset.endTime = parsedSeconds.toString();
        delete this.video.dataset.startTime;
        fragment = `t=0,${parsedSeconds}`;
      }
    }

    const newSrc = `${baseSrc}#${fragment}`;
    this.video.src = newSrc;

    if (targetFile && sourcePathForLink) {
      try {
        const MarkdownView = (this.app as any).workspace.getLeavesOfType('markdown')[0]?.view;
        if (MarkdownView && MarkdownView.file && (MarkdownView as any).editor) {
          const editor = (MarkdownView as any).editor;
          const { extractVideosFromMarkdownView } = require('../../video');
          const videos = extractVideosFromMarkdownView(MarkdownView);
          const match = videos.find((v: any) => v.file && v.file.path === targetFile.path);
          if (match) {
            const { position, linktext } = match;
            const { start, end } = position;
            const docText = editor.getValue();
            const before = docText.split('\n').slice(0, start.line).join('\n') + (start.line > 0 ? '\n' : '');
            const after = docText.split('\n').slice(end.line + 1).join('\n');
            const line = docText.split('\n')[start.line];
            const embedStart = start.col;
            const embedEnd = end.col;
            let newEmbed = linktext.replace(/#t=[^\]|]*/, '');
            newEmbed = newEmbed.replace(']]', `#${fragment}]]`);
            const newLine = line.slice(0, embedStart) + newEmbed + line.slice(embedEnd);
            const newDoc = [
              ...docText.split('\n').slice(0, start.line),
              newLine,
              ...docText.split('\n').slice(end.line + 1)
            ].join('\n');
            editor.setValue(newDoc);
            new Notice(`Embed link updated with timestamp.`);
          } else {
            new Notice(`Timestamp set, but could not find embed in markdown to update.`);
          }
        }
      } catch (e) {
        console.error('Failed to update embed link:', e);
        new Notice('Error updating embed link in markdown.');
      }
    }

    new Notice(`Video ${this.type} time set to ${parsedSeconds.toFixed(2)}s.`);
    this.close();
  }

  onClose() {
    this.contentEl.empty();
  }
}