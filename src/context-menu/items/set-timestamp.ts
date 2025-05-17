import { Menu, Notice, App, Modal, MarkdownView, Plugin } from 'obsidian';
import { getVideoLinkDetails, getCurrentTimeRounded } from '../utils';
import { parseTimestampToSeconds } from '../../timestamps/utils';
import { VideoWithTimestamp } from '../../video';
import { generateMarkdownLink } from 'obsidian-dev-utils/obsidian/Link';
import { reinitializeRestrictionHandlers } from '../../video/restriction-handler';

export function addSetStartTime(menu: Menu, plugin: Plugin, video: HTMLVideoElement) {
  menu.addItem(item =>
    item
      .setIcon('log-in')
      .setTitle('Set start time')
      .onClick(() => {
        new TimestampInputModal(plugin, video, 'start').open(); // Pass plugin directly
      })
  );
}

export function addSetEndTime(menu: Menu, plugin: Plugin, video: HTMLVideoElement) {
  menu.addItem(item =>
    item
      .setIcon('log-out')
      .setTitle('Set end time')
      .onClick(() => {
        new TimestampInputModal(plugin, video, 'end').open(); // Pass plugin directly
      })
  );
}

class TimestampInputModal extends Modal {
  pluginInstance: Plugin; // Store the plugin instance
  video: HTMLVideoElement;
  type: 'start' | 'end';
  textareaEl: HTMLTextAreaElement | null = null;

  constructor(plugin: Plugin, video: HTMLVideoElement, type: 'start' | 'end') { // Accept plugin instance
    super(plugin.app); // Pass app from plugin to super
    this.pluginInstance = plugin; // Store plugin instance
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
      initialValue = this.video.dataset.endTime;    } else {
      initialValue = currentTimestamp.toString();
    }
    const formattedCurrent = require('../../timestamps/utils').formatTimestamp(currentTimestamp);
    
    this.textareaEl = content.createEl('textarea', {
      cls: 'video-ts-textarea',
      attr: { rows: 1 }
    });
    this.textareaEl.value = initialValue;
    this.textareaEl.placeholder = `Enter ${this.type} time (e.g., 1:23 or 83.5)`;
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
      if(typeof endTime === 'number' && !isNaN(endTime)) this.video.dataset.endTime = endTime.toString(); else delete this.video.dataset.endTime;
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
      if(typeof startTime === 'number' && !isNaN(startTime)) this.video.dataset.startTime = startTime.toString(); else delete this.video.dataset.startTime;
    }

    const newSrcForDom = `${baseSrc}#${fragment}`;
    const newSrcToStoreInEditor = (targetFile && !isExternalFileUrl) 
        ? `${this.app.vault.getResourcePath(targetFile).split('#')[0]}#${fragment}` 
        : newSrcForDom;
    
    this.video.src = newSrcForDom;

    const mdView = this.app.workspace.getActiveViewOfType(MarkdownView);
    if (mdView && mdView.editor) {
      const editor = mdView.editor;
      try {
        const { extractVideosFromMarkdownView } = require('../../video');
        const allVideosInEditor: VideoWithTimestamp[] = extractVideosFromMarkdownView(mdView);
        let updatedInMarkdown = false;        const domVideoElements = Array.from(mdView.contentEl.querySelectorAll('video'));
        const currentVideoDomIndex = domVideoElements.indexOf(this.video);
        
        if (currentVideoDomIndex !== -1 && currentVideoDomIndex < allVideosInEditor.length) {
          const currentVideoInfo = allVideosInEditor[currentVideoDomIndex];
          // Create a loggable version of currentVideoInfo to avoid circular JSON errors
          const loggableVideoInfo = {
            ...currentVideoInfo,
            file: currentVideoInfo.file ? currentVideoInfo.file.path : null // Replace TFile with its path for logging
          };          // Only log in development builds
          if (process.env.NODE_ENV !== 'production') {
            console.log("TimestampInputModal: Matched video by index:", JSON.parse(JSON.stringify(loggableVideoInfo)));
          }

          // Use currentVideoInfo.type to determine how to update
          if ((currentVideoInfo.type === 'wiki' || currentVideoInfo.type === 'md') && currentVideoInfo.file) {
            const { line: startLine, col: startCol } = currentVideoInfo.position.start;
            const { line: endLine, col: endCol } = currentVideoInfo.position.end;
            const editorPosStart = { line: startLine, ch: startCol };
            const editorPosEnd = { line: endLine, ch: endCol };
            
            const newFullMdLink = generateMarkdownLink({
              app: this.app,
              targetPathOrFile: currentVideoInfo.file,
              sourcePathOrFile: mdView.file ?? '', // The file containing the link; fallback to empty string if null
              subpath: `#${fragment}`,
              isEmbed: currentVideoInfo.isEmbedded, // Use the stored isEmbedded property
              originalLink: currentVideoInfo.linktext, // originalLink helps preserve alias and link style
              alias: currentVideoInfo.alias // Pass the stored alias
            });
            
            editor.replaceRange(newFullMdLink, editorPosStart, editorPosEnd);
            new Notice(`Markdown embed link updated with timestamp.`);
            updatedInMarkdown = true;

          } else if (currentVideoInfo.type === 'html') {
            const { line: startLine, col: startCh } = currentVideoInfo.position.start; // ch for consistency with editor
            const { line: endLine, col: endCh } = currentVideoInfo.position.end;   // ch for consistency with editor
            const htmlLinkText = currentVideoInfo.linktext;

            // The editor positions for the full block might span multiple lines
            // htmlStart and htmlEnd from currentVideoInfo.position are character offsets within their respective lines.
            const blockStartPos = { line: startLine, ch: 0 }; // Start of the line where HTML block begins
            const blockEndPos = { line: endLine, ch: editor.getLine(endLine).length }; // End of the line where HTML block ends

            if (/^\s*<video[\s>]/i.test(htmlLinkText)) { // Test against the reliable linktext
              const currentBlockText = editor.getRange(blockStartPos, blockEndPos);
              const blockLines = currentBlockText.split('\n');
              let lineIdxInBlock = -1;              for (let i = 0; i < blockLines.length; i++) {
                if (blockLines[i].trim().match(/<video\s[^>]*src=/i)) {
                  lineIdxInBlock = i;
                  break;
                }
              }
              
              if (lineIdxInBlock !== -1) {
                blockLines[lineIdxInBlock] = blockLines[lineIdxInBlock].replace(
                  /src=("|')[^"'#]+(#[^"']*)?("|')/i,
                  `src="${newSrcToStoreInEditor}"`
                );
                editor.replaceRange(blockLines.join('\n'), blockStartPos, blockEndPos);
                new Notice('HTML video tag updated with timestamp.');
                updatedInMarkdown = true;
              } else {
                if (process.env.NODE_ENV !== 'production') {
                  console.warn("TimestampInputModal: Found HTML video block but couldn't find src attribute line.", currentVideoInfo, blockLines);
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
        }        new Notice('Error updating embed link in markdown: ' + e.message);
      }
    }
    
    new Notice(`Video ${this.type} time set to ${parsedSeconds.toFixed(2)}s.`);

    // Access plugin instance directly
    // @ts-ignore - Assuming 'settings' exists on the plugin instance. 
    // It would be better to cast to a specific plugin type that guarantees 'settings'
    // e.g., if VideoTimestampsPlugin is the actual class: const currentPlugin = this.pluginInstance as VideoTimestampsPlugin;
    if (this.pluginInstance && (this.pluginInstance as any).settings) {
      // Reinitialize video restriction handlers to apply the new timestamp
      reinitializeRestrictionHandlers((this.pluginInstance as any).settings);
    }
    
    this.close(); // Fixed typo from this.close();his.close();
  }

  onClose() {
    this.contentEl.empty();
  }
}