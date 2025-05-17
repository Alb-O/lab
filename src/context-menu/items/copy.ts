import { Menu, Notice } from 'obsidian';
import { formatTimestamp } from '../../timestamps/utils';
import { generateMarkdownLink } from 'obsidian-dev-utils/obsidian/Link';
import { getVideoLinkDetails } from '../utils/link-retriever';

// Helper to get video current time rounded to 2 decimal places if needed
function getCurrentTimeRounded(video: HTMLVideoElement): number {
    const t = video.currentTime;
    // if time is effectively integer, return as is
    if (Math.abs(t - Math.round(t)) < Number.EPSILON) return t;
    return Math.round(t * 100) / 100;
}


export function addCopyEmbedLink(menu: Menu, app: any, video: HTMLVideoElement) {
  menu.addItem(item =>
    item
      .setIcon('copy')
      .setTitle('Copy embed link')
      .onClick(() => {
        const linkDetails = getVideoLinkDetails(app, video);
        if (!linkDetails) {
          new Notice('Cannot copy link: View type not supported or active leaf not found.');
          return;
        }
        const { targetFile, sourcePathForLink, originalVideoSrcForNotice, isExternalFileUrl, externalFileUrl, attributesString } = linkDetails;
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
            app: app,
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
          .catch(err => {
            console.error('Failed to copy link: ', err);
            new Notice('Failed to copy link to clipboard.');
          });
      })
  );
}

export function addCopyEmbedAtCurrentTime(menu: Menu, app: any, video: HTMLVideoElement) {
  menu.addItem(item =>
    item
      .setIcon('copy')
      .setTitle('Copy embed at current time')
      .onClick(() => {
        const currentTime = getCurrentTimeRounded(video);
        const formattedTime = formatTimestamp(currentTime);
        const linkDetails = getVideoLinkDetails(app, video);
        if (!linkDetails) {
          new Notice('Cannot copy link: View type not supported or active leaf not found.');
          return;
        }
        const { targetFile, sourcePathForLink, originalVideoSrcForNotice, isExternalFileUrl, externalFileUrl, attributesString } = linkDetails;
        if (!targetFile && !isExternalFileUrl) {
          new Notice(`Video file not found. Source: ${originalVideoSrcForNotice || 'unknown'}`);
          return;
        }
        let linkText: string;
        if (isExternalFileUrl && externalFileUrl) {
          const baseSrc = externalFileUrl.split('#')[0];
          const newSrcWithTimestamp = `${baseSrc}#t=${currentTime}`;
          linkText = `<video src="${newSrcWithTimestamp}"${attributesString}></video>`;
        } else if (targetFile) {
          const timestampParam = `#t=${currentTime}`;
          linkText = generateMarkdownLink({
            app: app,
            targetPathOrFile: targetFile,
            sourcePathOrFile: sourcePathForLink,
            subpath: timestampParam,
            alias: formattedTime,
            isEmbed: true
          });
        } else {
          new Notice('Could not determine link type.');
          return;
        }
        navigator.clipboard.writeText(linkText)
          .then(() => {
            new Notice(`Copied link with timestamp (${formattedTime}).`);
          })
          .catch(err => {
            console.error('Failed to copy link: ', err);
            new Notice('Failed to copy link to clipboard.');
          });
      })
  );
}
