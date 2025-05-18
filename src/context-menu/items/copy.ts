import { Menu, Notice, Plugin } from 'obsidian';
import { formatFragment } from '../../fragments/utils';
import { generateMarkdownLink } from 'obsidian-dev-utils/obsidian/Link';
import { getVideoLinkDetails, getCurrentTimeRounded } from '../utils';
import { VideoFragmentsSettings } from '../../settings';

export function addCopyEmbedLink(menu: Menu, plugin: Plugin, video: HTMLVideoElement) {
    menu.addItem(item =>
        item
            .setIcon('copy')
            .setTitle('Copy embed link')
            .onClick(() => {
                const linkDetails = getVideoLinkDetails(plugin.app, video);
                if (!linkDetails) {
                    new Notice('Cannot copy link: View type not supported or active leaf not found.');
                    return;
                }

                const {
                    targetFile,
                    sourcePathForLink,
                    originalVideoSrcForNotice,
                    isExternalFileUrl,
                    externalFileUrl,
                    attributesString
                } = linkDetails;

                if (!targetFile && !isExternalFileUrl) {
                    new Notice(`Video file not found. Source: ${originalVideoSrcForNotice || 'unknown'}`);
                    return;
                }

                let linkText: string;

                if (isExternalFileUrl && externalFileUrl) {
                    const baseSrc = externalFileUrl.split('#')[0];
                    linkText = `<video src="${baseSrc}"${attributesString}></video>`;
                } else if (targetFile) {
                    linkText = generateMarkdownLink({
                        app: plugin.app,
                        targetPathOrFile: targetFile,
                        sourcePathOrFile: sourcePathForLink,
                        isEmbed: true
                    });
                } else {
                    new Notice('Could not determine link type.');
                    return;
                }

                navigator.clipboard.writeText(linkText)
                    .then(() => {
                        new Notice('Copied embed link.');
                    })
                    .catch((e) => {
                        new Notice(`Failed to copy link to clipboard: ${e instanceof Error ? e.message : String(e)}`);
                    });
            })
    );
}

export function addCopyEmbedAtCurrentTime(menu: Menu, plugin: Plugin, settings: VideoFragmentsSettings, video: HTMLVideoElement) {
    menu.addItem(item => item
        .setIcon('copy')
        .setTitle('Copy embed at current time')
        .onClick(() => {
            const currentTime = getCurrentTimeRounded(video);
            // Use user-defined settings for fragment formatting;
            const formattedFragment = formatFragment(currentTime, undefined, settings );
            const linkDetails = getVideoLinkDetails(plugin.app, video);
            if (!linkDetails) {
                new Notice('Cannot copy link: View type not supported or active leaf not found.');
                return;
            }

            const {
                targetFile,
                sourcePathForLink,
                originalVideoSrcForNotice,
                isExternalFileUrl,
                externalFileUrl,
                attributesString
            } = linkDetails;

            if (!targetFile && !isExternalFileUrl) {
                new Notice(`Video file not found. Source: ${originalVideoSrcForNotice || 'unknown'}`);
                return;
            }

            let linkText: string;

            if (isExternalFileUrl && externalFileUrl) {
                const baseSrc = externalFileUrl.split('#')[0];
                const newSrcWithFragment = `${baseSrc}#t=${formattedFragment}`;
                linkText = `<video src="${newSrcWithFragment}"${attributesString}></video>`;
            } else if (targetFile) {
                const fragmentParam = `#t=${formattedFragment}`;
                linkText = generateMarkdownLink({
                    app: plugin.app,
                    targetPathOrFile: targetFile,
                    sourcePathOrFile: sourcePathForLink,
                    subpath: fragmentParam,
                    alias: formattedFragment,
                    isEmbed: true
                });
            } else {
                new Notice('Could not determine link type.');
                return;
            }

            navigator.clipboard.writeText(linkText)
                .then(() => {
                    new Notice(`Copied link with fragment (${formattedFragment}).`);
                })
                .catch(e => {
                    new Notice(`Failed to copy link to clipboard: ${e instanceof Error ? e.message : String(e)}`);
                });
        })
    );
}
