import { Menu, Notice, Plugin, MarkdownView } from 'obsidian';
import { formatFragment } from '../../fragments/utils';
import { getCurrentTimeRounded, removeVideoEmbedByIndex, copyGeneric } from '../utils';
import { VideoFragmentsSettings } from '../../settings';

export function addEmbedActionsCommands(menu: Menu, plugin: Plugin, settings: VideoFragmentsSettings, video: HTMLVideoElement) {
    menu.addItem(item => item
        .setIcon('copy')
        .setTitle('Copy embed link')
        .setSection('vfrag-embed-actions')
        .onClick(() => copyGeneric(video, plugin.app, 'Copied embed link.'))
    );
    menu.addItem(item => item
        .setIcon('copy-plus')
        .setTitle('Copy embed and existing fragment')
        .setSection('vfrag-embed-actions')
        .onClick(() => {
            const { startTimeRaw: startRaw, endTimeRaw: endRaw } = video.dataset;
            if (!startRaw && !endRaw) {
                return copyGeneric(video, plugin.app, 'Copied embed link.');
            }
            const fragment = startRaw && endRaw ? `${startRaw},${endRaw}` : (startRaw || endRaw)!;
            copyGeneric(video, plugin.app, 'Copied embed with existing fragment.', fragment, fragment);
        })
    );
    menu.addItem(item => item
        .setIcon('flag-triangle-right')
        .setTitle('Copy embed starting at current time')
        .setSection('vfrag-embed-actions')
        .onClick(() => {
            const formatted = formatFragment(getCurrentTimeRounded(video), undefined, settings);
            copyGeneric(video, plugin.app, `Copied link with start fragment (${formatted}).`, formatted, formatted);
        })
    );
    menu.addItem(item => item
        .setIcon('flag-triangle-left')
        .setTitle('Copy embed ending at current time')
        .setSection('vfrag-embed-actions')
        .onClick(() => {
            const formattedTime = formatFragment(getCurrentTimeRounded(video), undefined, settings);
            const alias = `0,${formattedTime}`;
            copyGeneric(video, plugin.app, `Copied link with end fragment (${alias}).`, undefined, alias, formattedTime);
        })
    );
    menu.addItem(item => item
        .setIcon('x')
        .setTitle('Remove embed from note')
        .setSection('vfrag-embed-actions')
        .onClick(async () => {
            const view = plugin.app.workspace.getActiveViewOfType(MarkdownView);
            if (!view) {
                new Notice('Removing embed links only works from a Markdown note.');
                return;
            } if (view.getMode() === 'preview') {
                new Notice('Cannot remove while in reading view.');
                return;
            }
            const els = view.contentEl.querySelectorAll('video');
            const idx = Array.from(els).indexOf(video);
            await removeVideoEmbedByIndex(view, idx);
        })
    );
}