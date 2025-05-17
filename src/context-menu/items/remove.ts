import { Menu, Notice, MarkdownView, Plugin } from 'obsidian';
import { extractVideosFromMarkdownView } from '../../video';

export function addRemoveEmbedLink(menu: Menu, plugin: Plugin, video: HTMLVideoElement) {
  menu.addItem(item =>
    item
      .setIcon('trash')
      .setTitle('Remove embed link')
      .onClick(async () => {
        const view = plugin.app.workspace.getActiveViewOfType(MarkdownView);
        if (!view) {
          new Notice('Removing embed links only works from a Markdown note.');
          return;
        } if (view.getMode() === 'preview') {
          new Notice('Cannot remove while in reading view.');
          return;
        }

        const videos = extractVideosFromMarkdownView(view);
        const els = view.contentEl.querySelectorAll('video');
        const idx = Array.from(els).indexOf(video);

        if (idx < 0 || idx >= videos.length) {
          return;
        }

        const target = videos[idx];
        const { start, end } = target.position;
        const editor = view.editor;
        const embedText = editor.getRange(
          { line: start.line, ch: start.col },
          { line: end.line, ch: end.col }
        );

        if (/^\s*<video[\s>]/i.test(embedText)) {
          // For HTML video tags, extractVideosFromMarkdownView currently sets start.line and end.line
          // to be the same line where the <video src=...> tag is found.
          // This removes that entire line.
          editor.replaceRange(
            '',
            { line: start.line, ch: 0 },
            { line: end.line + 1, ch: 0 } // Removes lines from start.line up to end.line inclusive
          );
        } else {
          // For non-HTML embeds (e.g., markdown links)
          editor.replaceRange(
            '',
            { line: start.line, ch: start.col },
            { line: end.line, ch: end.col }
          );
          if (editor.getLine(start.line).trim() === '') {
            editor.replaceRange(
              '',
              { line: start.line, ch: 0 },
              { line: start.line + 1, ch: 0 }
            );
          }
        }
      })
  );
}

export function addRemoveTimestampFromEmbedLink(menu: Menu, plugin: Plugin, video: HTMLVideoElement) {
  menu.addItem(item =>
    item
      .setIcon('clock')
      .setTitle('Remove timestamp from embed link')
      .onClick(async () => {
        const view = plugin.app.workspace.getActiveViewOfType(MarkdownView);
        if (!view) {
          new Notice('Removing timestamps only works from a Markdown note.');
          return;
        } if (view.getMode() === 'preview') {
          new Notice('Cannot remove timestamp while in reading view.');
          return;
        }

        const videos = extractVideosFromMarkdownView(view);
        const els = view.contentEl.querySelectorAll('video');
        const idx = Array.from(els).indexOf(video);

        if (idx < 0 || idx >= videos.length) {
          return;
        }

        const target = videos[idx];
        const { start, end } = target.position;
        const editor = view.editor;
        const embedText = editor.getRange(
          { line: start.line, ch: start.col },
          { line: end.line, ch: end.col }
        );
        
        const newEmbedText = embedText.replace(/([?&#]t=(\d+(?:\.\d+)?(,\d+(?:\.\d+)?)?|\d{1,2}:\d{2}(,\d{1,2}:\d{2})?))/, '');
        if (embedText !== newEmbedText) {
          editor.replaceRange(
            newEmbedText,
            { line: start.line, ch: start.col },
            { line: end.line, ch: end.col }
          );
        }
      })
  );
}