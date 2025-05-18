import { Menu, Notice, Plugin } from 'obsidian';
import { getVideoLinkDetails } from '../utils';

export function addOpenCommands(menu: Menu, plugin: Plugin, video: HTMLVideoElement) {
    menu.addItem(item => item
        .setIcon('lucide-file')
        .setTitle('Open link')
        .setSection('vfrag-open')
        .onClick(() => {
            const linkDetails = getVideoLinkDetails(plugin.app, video);
            if (!linkDetails) {
                new Notice('Cannot open video: View type not supported or active leaf not found.');
                return;
            }
            const { targetFile, originalVideoSrcForNotice, isExternalFileUrl, externalFileUrl } = linkDetails;
            if (!targetFile && !isExternalFileUrl) {
                new Notice(`Video file not found. Source: ${originalVideoSrcForNotice || 'unknown'}`);
                return;
            }
            if (isExternalFileUrl && externalFileUrl) {
                window.open(externalFileUrl.split('#')[0]);
            } else if (targetFile) {
                plugin.app.workspace.openLinkText(targetFile.path, '', false);
            } else {
                new Notice('Could not determine video to open.');
            }
        })
    );
    menu.addItem(item => item
        .setIcon('lucide-file-plus')
        .setTitle('Open in new tab')
        .setSection('vfrag-open')
        .onClick(() => {
            const linkDetails = getVideoLinkDetails(plugin.app, video);
            if (!linkDetails) {
                new Notice('Cannot open video: View type not supported or active leaf not found.');
                return;
            }
            const { targetFile, originalVideoSrcForNotice, isExternalFileUrl, externalFileUrl } = linkDetails;
            if (!targetFile && !isExternalFileUrl) {
                new Notice(`Video file not found. Source: ${originalVideoSrcForNotice || 'unknown'}`);
                return;
            }
            if (isExternalFileUrl && externalFileUrl) {
                window.open(externalFileUrl.split('#')[0]);
            } else if (targetFile) {
                plugin.app.workspace.openLinkText(targetFile.path, '', true);
            } else {
                new Notice('Could not determine video to open.');
            }
        })
    );
    menu.addItem(item => item
        .setIcon('lucide-separator-vertical')
        .setTitle('Open to the right')
        .setSection('vfrag-open')
        .onClick(() => {
            const linkDetails = getVideoLinkDetails(plugin.app, video);
            if (!linkDetails) {
                new Notice('Cannot open video: View type not supported or active leaf not found.');
                return;
            }
            const { targetFile, originalVideoSrcForNotice, isExternalFileUrl, externalFileUrl } = linkDetails;
            if (!targetFile && !isExternalFileUrl) {
                new Notice(`Video file not found. Source: ${originalVideoSrcForNotice || 'unknown'}`);
                return;
            } if (isExternalFileUrl && externalFileUrl) {
                window.open(externalFileUrl.split('#')[0]);
            } else if (targetFile) {
                plugin.app.workspace.openLinkText(targetFile.path, '', 'split');
            } else {
                new Notice('Could not determine video to open.');
            }
        })
    );
    menu.addItem(item => item
        .setIcon('lucide-picture-in-picture-2')
        .setTitle('Open in new window')
        .setSection('vfrag-open')
        .onClick(() => {
            const linkDetails = getVideoLinkDetails(plugin.app, video);
            if (!linkDetails) {
                new Notice('Cannot open video: View type not supported or active leaf not found.');
                return;
            }
            const { targetFile, originalVideoSrcForNotice, isExternalFileUrl, externalFileUrl } = linkDetails;
            if (!targetFile && !isExternalFileUrl) {
                new Notice(`Video file not found. Source: ${originalVideoSrcForNotice || 'unknown'}`);
                return;
            }

            if (isExternalFileUrl && externalFileUrl) {
                window.open(externalFileUrl.split('#')[0]);
            } else if (targetFile) {
                plugin.app.workspace.openLinkText(targetFile.path, '', 'window');
            } else {
                new Notice('Could not determine video to open.');
            }
        })
    );
}