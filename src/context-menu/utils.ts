import { TFile, MarkdownView, normalizePath, App, FileSystemAdapter, FileView, Notice } from 'obsidian';
import { extractVideosFromMarkdownView, VideoWithTimestamp } from '../video';
import { generateFragmentString, TempFragment, parseTimestampToSeconds } from '../timestamps/utils';
import { generateMarkdownLink } from 'obsidian-dev-utils/obsidian/Link';
import { VideoTimestampsSettings } from '../settings';

export interface VideoLinkDetails {
    targetFile: TFile | null;
    sourcePathForLink: string;
    originalVideoSrcForNotice: string | null;
    isExternalFileUrl: boolean;
    externalFileUrl: string | null; // Full src attribute for external file URLs
    attributesString: string; // String of filtered HTML attributes
}

export function getVideoLinkDetails(app: App, videoEl: HTMLVideoElement): VideoLinkDetails | null {
    // Original class list for attribute generation
    const originalClassList = Array.from(videoEl.classList).join(' ');

    const activeLeaf = app.workspace.activeLeaf;
    if (!activeLeaf) {
        return null;
    }

    let targetFile: TFile | null = null;
    let sourcePathForLink: string = '';
    const originalVideoSrcForNotice: string | null = videoEl.dataset.timestampPath || videoEl.currentSrc || videoEl.src;
    let isExternalFileUrl = false;
    let externalFileUrl: string | null = null;
    let attributesString: string = "";

    const excludedAttributes = [
        'data-controls-initialized', 'data-timestamp-path', 'data-context-menu-initialized',
        'data-start-time', 'data-end-time', 'data-start-time-percent', 'data-end-time-percent',
        'data-reached-end', 'data-seeked-past-end', 'data-auto-resume', 'data-should-auto-play',
        'data-user-paused', 'data-is-seeking', 'src' // src will be handled separately
    ];

    for (const attr of Array.from(videoEl.attributes)) {
        const attrNameLower = attr.name.toLowerCase();
        if (excludedAttributes.includes(attrNameLower)) {
            continue;
        }
        if (attrNameLower === 'class') {
            // Filter out video-ts-* and paused from the original class list for the new attribute string
            const filteredClasses = originalClassList.split(' ')
                .filter(cls => !cls.startsWith('video-ts-') && cls !== 'paused' && cls !== '')
                .join(' ');
            if (filteredClasses) {
                attributesString += ` class="${filteredClasses}"`;
            }
            continue;
        }
        if (attr.value === '') { // Boolean attribute
            attributesString += ` ${attr.name}`;
        } else {
            attributesString += ` ${attr.name}="${attr.value}"`;
        }
    }
    // If the original element had classes but not a class attribute (e.g. added via JS .classList.add)
    // and we haven't added a class attribute yet (e.g. because it wasn't in videoEl.attributes)
    // we should construct it from originalClassList
    if (!videoEl.hasAttribute('class') && !attributesString.includes(' class=')) {
        const filteredClasses = originalClassList.split(' ')
            .filter(cls => !cls.startsWith('video-ts-') && cls !== 'paused' && cls !== '')
            .join(' ');
        if (filteredClasses) {
            attributesString += ` class="${filteredClasses}"`;
        }
    }

    if (activeLeaf.view instanceof MarkdownView) {
        const mdView = activeLeaf.view;
        sourcePathForLink = mdView.file?.path || '';

        if (mdView.getMode() === 'preview') {
            const currentVideoSrc = videoEl.currentSrc || videoEl.src; // Prefer live currentSrc for HTML blocks
            if (currentVideoSrc) {
                if (currentVideoSrc.startsWith('file:///')) {
                    isExternalFileUrl = true;
                    externalFileUrl = currentVideoSrc;
                    targetFile = null; // No TFile for external URLs
                } else if (currentVideoSrc.startsWith('app://')) {
                    try {
                        const url = new URL(currentVideoSrc);
                        let absPathFromUrl = decodeURIComponent(url.pathname);

                        if (absPathFromUrl.startsWith('/') && absPathFromUrl.length > 1 && absPathFromUrl[1] !== ':') {
                            absPathFromUrl = absPathFromUrl.substring(1);
                        }
                        absPathFromUrl = normalizePath(absPathFromUrl);

                        if (app.vault.adapter instanceof FileSystemAdapter) {
                            const vaultBasePath = normalizePath(app.vault.adapter.getBasePath());
                            let attemptedRelativePathForLog: string = "";

                            if (absPathFromUrl.toLowerCase().startsWith(vaultBasePath.toLowerCase())) {
                                // Path is INSIDE the vault
                                let relPath = absPathFromUrl.substring(vaultBasePath.length);

                                if (relPath.startsWith('/') || relPath.startsWith('\\')) {
                                    relPath = relPath.substring(1);
                                }
                                attemptedRelativePathForLog = relPath;
                                if (relPath === "") {
                                    targetFile = null;
                                } else {
                                    const normalizedRelativePath = normalizePath(relPath);
                                    attemptedRelativePathForLog = normalizedRelativePath;
                                    if (normalizedRelativePath === '.') {
                                        targetFile = null;
                                    } else {
                                        targetFile = app.vault.getFileByPath(normalizedRelativePath);
                                    }
                                }
                                if (!targetFile) { // Log if still not found after attempting vault-relative resolution
                                    console.warn(`VideoTimestamps: Could not find TFile for app:// URL (inside vault). Attempted relative path: '${attemptedRelativePathForLog}'. Original src: ${currentVideoSrc}`);
                                }
                            } else {
                                // Path is OUTSIDE the vault, treat as external
                                console.warn(`VideoTimestamps: app:// URL path '${absPathFromUrl}' is outside the vault base path '${vaultBasePath}'. Converting to file:/// protocol. Original src: ${currentVideoSrc}`);
                                isExternalFileUrl = true;
                                let fileUrlPath = absPathFromUrl;
                                if (!absPathFromUrl.startsWith('/')) {
                                    fileUrlPath = '/' + absPathFromUrl;
                                }
                                externalFileUrl = `file://${fileUrlPath}`;
                                targetFile = null;
                            }
                        } else {
                            if (process.env.NODE_ENV !== 'production') {
                                console.warn(`VideoTimestamps: Vault adapter is not FileSystemAdapter, cannot resolve app:// URL. Converting to file:/// protocol. Original src: ${currentVideoSrc}`);
                            }
                            let fileUrlPath = absPathFromUrl;// absPathFromUrl was derived from URL(currentVideoSrc).pathname
                            if (!absPathFromUrl.startsWith('/')) {
                                fileUrlPath = '/' + absPathFromUrl;
                            }
                            externalFileUrl = `file://${fileUrlPath}`;
                            isExternalFileUrl = true;
                            targetFile = null;
                        }
                    } catch (e) {
                        if (process.env.NODE_ENV !== 'production') {
                            console.error('VideoTimestamps: Error parsing app:// URL for video path:', currentVideoSrc, e);
                        }
                        // Fallback: try to use the original src if it looks like a URL, otherwise null
                        try {
                            new URL(currentVideoSrc); // check if it's a valid URL
                            externalFileUrl = currentVideoSrc; // Keep original if it's a valid URL but failed parsing
                        } catch (urlError) {
                            externalFileUrl = null;
                        }
                        isExternalFileUrl = true;
                        targetFile = null;
                    }
                } else { // Not app:// or file://, assume vault-relative or needs getFirstLinkpathDest
                    const pathFromSrc = currentVideoSrc.split('#')[0];
                    const resolvedFile = app.metadataCache.getFirstLinkpathDest(pathFromSrc, sourcePathForLink);
                    if (resolvedFile instanceof TFile) {
                        targetFile = resolvedFile;
                    } else {
                        const normalizedDirectPath = normalizePath(pathFromSrc);
                        const foundFile = app.vault.getFileByPath(normalizedDirectPath);
                        if (foundFile instanceof TFile) {
                            targetFile = foundFile;
                        }
                    }
                }
            }
        } else { // Source or Live Preview mode
            const currentVideoSrc = videoEl.currentSrc || videoEl.src; // Check src directly for HTML blocks in editor
            if (currentVideoSrc) {
                if (currentVideoSrc.startsWith('file:///')) {
                    isExternalFileUrl = true;
                    externalFileUrl = currentVideoSrc;
                    targetFile = null;
                } else if (currentVideoSrc.startsWith('app://')) {
                    // Apply the same app:// logic as in preview mode
                    try {
                        const url = new URL(currentVideoSrc);
                        let absPathFromUrl = decodeURIComponent(url.pathname);

                        if (absPathFromUrl.startsWith('/') && absPathFromUrl.length > 1 && absPathFromUrl[1] !== ':') {
                            absPathFromUrl = absPathFromUrl.substring(1);
                        }
                        absPathFromUrl = normalizePath(absPathFromUrl);

                        if (app.vault.adapter instanceof FileSystemAdapter) {
                            const vaultBasePath = normalizePath(app.vault.adapter.getBasePath());
                            let attemptedRelativePathForLog: string = "";

                            if (absPathFromUrl.toLowerCase().startsWith(vaultBasePath.toLowerCase())) {
                                // Path is INSIDE the vault
                                let relPath = absPathFromUrl.substring(vaultBasePath.length);

                                if (relPath.startsWith('/') || relPath.startsWith('\\')) {
                                    relPath = relPath.substring(1);
                                }
                                attemptedRelativePathForLog = relPath;
                                if (relPath === "") {
                                    targetFile = null;
                                } else {
                                    const normalizedRelativePath = normalizePath(relPath);
                                    attemptedRelativePathForLog = normalizedRelativePath;
                                    if (normalizedRelativePath === '.') {
                                        targetFile = null;
                                    } else {
                                        targetFile = app.vault.getFileByPath(normalizedRelativePath);
                                    }
                                } if (!targetFile) {
                                    if (process.env.NODE_ENV !== 'production') {
                                        console.warn(`VideoTimestamps: Could not find TFile for app:// URL (inside vault). Attempted relative path: '${attemptedRelativePathForLog}'. Original src: ${currentVideoSrc}`);
                                    }
                                }
                            } else {
                                // Path is OUTSIDE the vault, treat as external
                                if (process.env.NODE_ENV !== 'production') {
                                    console.warn(`VideoTimestamps: app:// URL path '${absPathFromUrl}' is outside the vault base path '${vaultBasePath}'. Converting to file:/// protocol. Original src: ${currentVideoSrc}`);
                                }
                                isExternalFileUrl = true;
                                let fileUrlPath = absPathFromUrl;
                                if (!absPathFromUrl.startsWith('/')) {
                                    fileUrlPath = '/' + absPathFromUrl;
                                }
                                externalFileUrl = `file://${fileUrlPath}`;
                                targetFile = null;
                            }
                        } else {
                            if (process.env.NODE_ENV !== 'production') {
                                console.warn(`VideoTimestamps: Vault adapter is not FileSystemAdapter, cannot resolve app:// URL. Converting to file:/// protocol. Original src: ${currentVideoSrc}`);
                            }
                            let fileUrlPath = absPathFromUrl;
                            if (!absPathFromUrl.startsWith('/')) {
                                fileUrlPath = '/' + absPathFromUrl;
                            }
                            externalFileUrl = `file://${fileUrlPath}`;
                            isExternalFileUrl = true;
                            targetFile = null;
                        }
                    } catch (e) {
                        if (process.env.NODE_ENV !== 'production') {
                            console.error('VideoTimestamps: Error parsing app:// URL for video path:', currentVideoSrc, e);
                        }
                        isExternalFileUrl = true;
                        try {
                            new URL(currentVideoSrc);
                            externalFileUrl = currentVideoSrc;
                        } catch (urlError) {
                            externalFileUrl = null;
                        }
                        targetFile = null;
                    }
                } else {
                    // Not a file:/// or app:// src, proceed with Markdown metadata matching
                    isExternalFileUrl = false;
                    const videosMeta = extractVideosFromMarkdownView(mdView);
                    const els = mdView.contentEl.querySelectorAll('video');
                    const idx = Array.from(els).indexOf(videoEl);
                    if (idx >= 0 && idx < videosMeta.length) {
                        const videoMetaPath = videosMeta[idx].path;
                        const resolvedFile = app.vault.getAbstractFileByPath(videoMetaPath);
                        if (resolvedFile instanceof TFile) {
                            targetFile = resolvedFile;
                        }
                    }
                }
            } else {
                // No currentVideoSrc in editor mode, try metadata matching as a fallback
                isExternalFileUrl = false;
                const videosMeta = extractVideosFromMarkdownView(mdView);
                const els = mdView.contentEl.querySelectorAll('video');
                const idx = Array.from(els).indexOf(videoEl);
                if (idx >= 0 && idx < videosMeta.length) {
                    const videoMetaPath = videosMeta[idx].path;
                    const resolvedFile = app.vault.getAbstractFileByPath(videoMetaPath);
                    if (resolvedFile instanceof TFile) {
                        targetFile = resolvedFile;
                    }
                }
            }
        }
    } else if (activeLeaf.view instanceof FileView && activeLeaf.view.getViewType() === 'video') {
        if (activeLeaf.view.file instanceof TFile) {
            targetFile = activeLeaf.view.file;
            sourcePathForLink = '';
            isExternalFileUrl = false;
        }
    } else {
        return null;
    }

    return { targetFile, sourcePathForLink, originalVideoSrcForNotice, isExternalFileUrl, externalFileUrl, attributesString };
}

// Helper to get video current time rounded to 2 decimal places if needed
export function getCurrentTimeRounded(video: HTMLVideoElement): number {
    const t = video.currentTime;
    return parseFloat((Math.round(t * 100) / 100).toFixed(2));
}

// Helper to apply a TempFragment to a video element, updating dataset attributes and src
export function applyFragmentToVideo(video: HTMLVideoElement, fragment: TempFragment | null): void {
    // Construct base URL without fragment
    const currentSrcUrl = new URL(video.currentSrc || video.src);
    const baseSrc = `${currentSrcUrl.protocol}//${currentSrcUrl.host}${currentSrcUrl.pathname}${currentSrcUrl.search}`;

    // Clear existing timestamp datasets
    delete video.dataset.startTimeRaw;
    delete video.dataset.startTime;
    delete video.dataset.endTimeRaw;
    delete video.dataset.endTime;

    if (fragment) {
        if (fragment.startRaw) video.dataset.startTimeRaw = fragment.startRaw;
        // Only set startTime if it's >= 0 and not 0.001 (remove 0.001 as a placeholder)
        if (fragment.start !== undefined && fragment.start >= 0 && fragment.start !== 0.001) video.dataset.startTime = fragment.start.toString();
        if (fragment.endRaw) video.dataset.endTimeRaw = fragment.endRaw;
        if (fragment.end !== undefined && fragment.end >= 0) video.dataset.endTime = fragment.end.toString();
    }

    // Update src with new fragment
    const fragmentString = fragment ? generateFragmentString(fragment) : '';
    video.src = `${baseSrc}${fragmentString ? `#${fragmentString}` : ''}`;
}

// New function to add
export async function setAndSaveVideoFragment(
    app: App,
    video: HTMLVideoElement,
    settings: VideoTimestampsSettings, // Kept for potential future settings-dependent logic
    newFragment: TempFragment | null
): Promise<boolean> {
    // 1. Apply to video element (dataset and src)
    applyFragmentToVideo(video, newFragment); // This is synchronous

    // 2. Update editor link
    const mdView = app.workspace.getActiveViewOfType(MarkdownView);
    if (mdView && mdView.editor && mdView.file) {
        try {
            const allVideoElementsInView = mdView.contentEl.querySelectorAll('video');
            await updateEditorLinkInFile(app, video, mdView.file, newFragment, allVideoElementsInView);
            return true;
        } catch (e: any) {
            if (process.env.NODE_ENV !== 'production') {
                console.error('VideoTimestamps: Failed to update embed link in setAndSaveVideoFragment:', e);
            }
            new Notice(`Error updating editor link: ${e.message}`);
            return false;
        }
    } else if (mdView && mdView.editor && !mdView.file) {
        new Notice('Timestamps applied to video. Save the file to update the link.');
        return true; 
    } else {
        // Not in a markdown view with an editor, or no file (e.g., viewing a video file directly)
        // Fragment is applied to the video, nothing else to do.
        return true;
    }
}

export async function updateEditorLinkInFile(
    app: App,
    videoEl: HTMLVideoElement, // The specific video element being modified
    currentFile: TFile, // The file containing the link
    newFragment: TempFragment | null, // The new fragment to apply
    allVideoElementsInView: NodeListOf<HTMLVideoElement> // All video elements in the current view, for indexing
): Promise<void> {
    const { extractVideosFromMarkdownView } = require('../video'); // Local require to avoid circular deps if any at top level
    const allVideosInEditor: VideoWithTimestamp[] = extractVideosFromMarkdownView(app.workspace.getActiveViewOfType(MarkdownView)!);
    const currentVideoDomIndex = Array.from(allVideoElementsInView).indexOf(videoEl);
    const fragmentString = newFragment ? generateFragmentString(newFragment) : '';

    if (currentVideoDomIndex !== -1 && currentVideoDomIndex < allVideosInEditor.length) {
        const currentVideoInfo = allVideosInEditor[currentVideoDomIndex];
        const subpath = fragmentString ? `#${fragmentString}` : '';

        if ((currentVideoInfo.type === 'wiki' || currentVideoInfo.type === 'md') && currentVideoInfo.file) {
            const { line: startLine, col: startCol } = currentVideoInfo.position.start;
            const { line: endLine, col: endCol } = currentVideoInfo.position.end;

            const newFullMdLink = generateMarkdownLink({
                app: app,
                targetPathOrFile: currentVideoInfo.file,
                sourcePathOrFile: currentFile,
                subpath: subpath,
                isEmbed: currentVideoInfo.isEmbedded,
                originalLink: currentVideoInfo.linktext,
                alias: currentVideoInfo.alias
            });

            const fileContent = await app.vault.read(currentFile);
            const lines = fileContent.split('\n');
            if (startLine === endLine) {
                lines[startLine] = lines[startLine].substring(0, startCol) + newFullMdLink + lines[startLine].substring(endCol);
            } else {
                const prefix = lines[startLine].substring(0, startCol);
                const suffix = lines[endLine].substring(endCol);
                const combinedLine = prefix + newFullMdLink + suffix;
                lines.splice(startLine, endLine - startLine + 1, combinedLine);
            }
            await app.vault.modify(currentFile, lines.join('\n'));
        } else if (currentVideoInfo.type === 'html') {
            const { line: startLineHtml, col: startChHtml } = currentVideoInfo.position.start;
            const { line: endLineHtml, col: endChHtml } = currentVideoInfo.position.end;
            const htmlLinkText = currentVideoInfo.linktext; // This is the full HTML block
            const blockStartPosLine = startLineHtml;
            const blockEndPosLine = endLineHtml;

            if (/^\s*<video[\s>]/i.test(htmlLinkText)) {
                const fileContent = await app.vault.read(currentFile);
                const lines = fileContent.split('\n');
                const originalBlockLines = lines.slice(blockStartPosLine, blockEndPosLine + 1);
                let lineIdxInBlock = -1; // Index within the originalBlockLines
                let actualLineNumber = -1; // Actual line number in the file

                // Find the line with the src attribute within the sliced block
                for (let i = 0; i < originalBlockLines.length; i++) {
                    if (originalBlockLines[i].trim().match(/<video\s[^>]*src=/i)) {
                        lineIdxInBlock = i;
                        actualLineNumber = blockStartPosLine + i;
                        break;
                    }
                }

                if (lineIdxInBlock !== -1 && actualLineNumber !== -1) {
                    let baseVideoSrc = "";
                    const srcMatch = originalBlockLines[lineIdxInBlock].match(/src=("|')([^"'#]+)/i);
                    if (srcMatch && srcMatch[2]) {
                        baseVideoSrc = srcMatch[2];
                    } else {
                        // If src is not found or not in expected format, we might have an issue.
                        // For safety, try to get it from videoEl.dataset.timestampPath or a cleaned version of videoEl.src
                        const currentSrcUrl = new URL(videoEl.dataset.timestampPath || videoEl.currentSrc || videoEl.src);
                        baseVideoSrc = `${currentSrcUrl.protocol}//${currentSrcUrl.host}${currentSrcUrl.pathname}${currentSrcUrl.search}`;
                    }

                    const newHtmlSrcAttr = `src="${baseVideoSrc}${subpath}"`;
                    const modifiedLine = originalBlockLines[lineIdxInBlock].replace(
                        /src=("|')[^"'#]+(#[^"']*)?("|')/i,
                        newHtmlSrcAttr
                    );
                    
                    // Update the specific line in the main lines array
                    lines[actualLineNumber] = modifiedLine;
                    await app.vault.modify(currentFile, lines.join('\n'));
                } else {
                    if (process.env.NODE_ENV !== 'production') {
                        console.warn('VideoTimestamps: Could not find <video src="..."> line in HTML block to update.', currentVideoInfo);
                    }
                    throw new Error('Could not find <video src="..."> line in HTML block.');
                }
            } else {
                 if (process.env.NODE_ENV !== 'production') {
                    console.warn('VideoTimestamps: HTML block does not start with <video>. Cannot update automatically.', currentVideoInfo);
                 }
                 throw new Error('HTML block does not start with <video>.');
            }
        } else {
            if (process.env.NODE_ENV !== 'production') {
                console.warn('VideoTimestamps: Video type not recognized or file missing for editor link update.', currentVideoInfo);
            }
            throw new Error('Video type not recognized or file missing.');
        }
    } else {
        if (process.env.NODE_ENV !== 'production') {
            console.warn('VideoTimestamps: Video element not found in editor metadata. DOM index:', currentVideoDomIndex, 'Editor videos count:', allVideosInEditor.length);
        }
        throw new Error('Video element not found in editor metadata.');
    }
}

export async function processTimestampAction(
    app: App,
    video: HTMLVideoElement,
    action: 'set' | 'clear',
    timestampType: 'start' | 'end',
    settings: VideoTimestampsSettings,
    originalFragment: TempFragment | null,
    rawInputValue?: string // Only for 'set' action
): Promise<boolean> {
    const linkDetails = getVideoLinkDetails(app, video);
    if (!linkDetails) {
        new Notice('Error: Could not retrieve video details to update the source.');
        return false;
    }

    let newFragment: TempFragment | null = null;
    let noticeMessage = '';

    if (action === 'set') {
        if (!rawInputValue) {
            new Notice('Timestamp cannot be empty.');
            return false;
        }
        const parsedSeconds = parseTimestampToSeconds(rawInputValue);
        if (parsedSeconds === null) {
            new Notice('Invalid timestamp format. Use seconds (e.g., 65.5), mm:ss (e.g., 1:05.5), or special values like start/end.');
            return false;
        }

        if (timestampType === 'start') {
            const currentEndTime = originalFragment?.end;
            if (typeof currentEndTime === 'number' && currentEndTime >= 0 && parsedSeconds >= currentEndTime) {
                new Notice('Start time cannot be after or equal to the end time.');
                return false;
            }
            newFragment = {
                start: parsedSeconds,
                startRaw: rawInputValue,
                end: currentEndTime !== undefined && currentEndTime >= 0 ? currentEndTime : -1,
                endRaw: originalFragment?.endRaw
            };
        } else { // type === 'end'
            const currentStartTime = originalFragment?.start;
            if (typeof currentStartTime === 'number' && currentStartTime >= 0 && parsedSeconds <= currentStartTime) {
                new Notice('End time cannot be before or equal to the start time.');
                return false;
            }
            newFragment = {
                start: currentStartTime !== undefined && currentStartTime >= 0 ? currentStartTime : -1,
                startRaw: originalFragment?.startRaw,
                end: parsedSeconds,
                endRaw: rawInputValue
            };
        }
        noticeMessage = `Video ${timestampType} time set to ${rawInputValue}.`;

    } else { // action === 'clear'
        if (timestampType === 'start') {
            if (originalFragment) {
                newFragment = { ...originalFragment, start: -1, startRaw: undefined };
                if (newFragment.end < 0 && !newFragment.endRaw) newFragment = null;
            }
        } else { // type === 'end'
            if (originalFragment) {
                newFragment = { ...originalFragment, end: -1, endRaw: undefined };
                if (newFragment.start < 0 && !newFragment.startRaw) newFragment = null;
                else if (newFragment.start === 0 && !newFragment.startRaw && newFragment.end < 0) newFragment = null;
            }
        }
        // Clean up fragment if it becomes t=0 due to clearing
        if (newFragment && newFragment.start === 0 && !newFragment.startRaw && newFragment.end < 0 && !newFragment.endRaw) {
            newFragment = null;
        }
        if (newFragment && newFragment.end === 0 && !newFragment.endRaw && newFragment.start < 0 && !newFragment.startRaw) {
            newFragment = null;
        }
        noticeMessage = `Video ${timestampType} time cleared.`;
    }

    applyFragmentToVideo(video, newFragment);

    const mdView = app.workspace.getActiveViewOfType(MarkdownView);
    if (mdView && mdView.editor && mdView.file) {
        try {
            await updateEditorLinkInFile(app, video, mdView.file, newFragment, mdView.contentEl.querySelectorAll('video'));
            new Notice(noticeMessage);
        } catch (e: any) {
            if (process.env.NODE_ENV !== 'production') {
                console.error('Failed to update embed link:', e);
            }
            new Notice(`Error updating embed link: ${e.message}`);
            // Even if link update fails, the video element itself was updated, so don't necessarily return false from processTimestampAction
            // The notice about the error should be sufficient.
        }
    } else if (mdView && mdView.editor && !mdView.file) {
        new Notice(`${noticeMessage} Cannot update link: current file is not saved.`);
    } else {
        new Notice(`${noticeMessage} Could not update markdown link (no active file/editor).`);
    }
    return true; // Indicates the primary action (setting/clearing timestamp on video) was attempted/done.
}
