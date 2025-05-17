import { Menu, Notice, MarkdownView } from 'obsidian';
import { extractVideosFromMarkdownView } from '../../video';

export function addRemoveEmbedLink(menu: Menu, app: any, video: HTMLVideoElement) {
  menu.addItem(item =>
    item
      .setIcon('trash')
      .setTitle('Remove embed link')
      .onClick(async () => {
        const view = this.app.workspace.getActiveViewOfType(MarkdownView);
        if (!view) {
          new Notice('Removing embed links only works from a Markdown note.');
          return;
        }
        // prevent removal in preview (reading) mode
        if (view.getMode() === 'preview') {
          new Notice('Cannot remove while in reading view.');
          return;
        }
        const videos = extractVideosFromMarkdownView(view);

        // Match this <video> element to its metadata by index
        const els = view.contentEl.querySelectorAll('video');
        const idx = Array.from(els).indexOf(video);
        if (idx < 0 || idx >= videos.length) return;
        const target = videos[idx];

        // Remove only the specific embed link at position
        const { start, end } = target.position;
        const editor = view.editor;
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
      })
  );
}
