import { TFile, MarkdownView, normalizePath, App, FileSystemAdapter, FileView, Notice } from 'obsidian';
import { markdownExtractor, VideoWithFragment } from '@markdown';
import { generateMarkdownLink } from 'obsidian-dev-utils/obsidian/Link';
import { generateFragmentString, TempFragment, parseFragmentToSeconds, debug } from '@utils';
import { FragmentsSettings } from '@settings';

export interface VideoLinkDetails {
    targetFile: TFile | null;
    sourcePathForLink: string;
    originalVideoSrcForNotice: string | null;
    isExternalFileUrl: boolean;
    externalFileUrl: string | null; // Full src attribute for external file URLs
    attributesString: string; // String of filtered HTML attributes
}

// Helper type for unified source resolution
interface SourceResolution {
    targetFile: TFile | null;
    isExternalFileUrl: boolean;
    externalFileUrl: string | null;
}

/**
 * Resolve a video source URL to either a vault file or external URL.
 */
function resolveVideoSource(
    src: string,
    app: App,
    sourcePathForLink: string
): SourceResolution {
    // File URL
    if (src.startsWith('file:///')) {
        return { targetFile: null, isExternalFileUrl: true, externalFileUrl: src };
    }
    // App URL (vault-relative)
    if (src.startsWith('app://')) {
        try {
            const url = new URL(src);
            let path = decodeURIComponent(url.pathname.replace(/^\//, ''));
            const normalized = normalizePath(path);
            const file = app.vault.getFileByPath(normalized);
            return file instanceof TFile
                ? { targetFile: file, isExternalFileUrl: false, externalFileUrl: null }
                : { targetFile: null, isExternalFileUrl: true, externalFileUrl: `file://${normalized}` };
        } catch {
            return { targetFile: null, isExternalFileUrl: true, externalFileUrl: src };
        }
    }
    // Markdown-relative link
    const pathPart = src.split('#')[0];
    const mdFile = app.metadataCache.getFirstLinkpathDest(pathPart, sourcePathForLink);
    if (mdFile instanceof TFile) {
        return { targetFile: mdFile, isExternalFileUrl: false, externalFileUrl: null };
    }
    // Fallback: treat as external URL if valid
    try {
        new URL(src);
        return { targetFile: null, isExternalFileUrl: true, externalFileUrl: src };
    } catch {
        return { targetFile: null, isExternalFileUrl: false, externalFileUrl: null };
    }
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
    const originalVideoSrcForNotice: string | null = videoEl.dataset.fragmentPath || videoEl.currentSrc || videoEl.src;
    let isExternalFileUrl = false;
    let externalFileUrl: string | null = null;
    let attributesString: string = "";

    const excludedAttributes = [
        'data-controls-initialized', 'data-fragment-path', 'data-context-menu-initialized',
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
            // Filter out vfrag-* and paused from the original class list for the new attribute string
            const filteredClasses = originalClassList.split(' ')
                .filter(cls => !cls.startsWith('vfrag-') && cls !== 'paused' && cls !== '')
                .join(' ');
            if (filteredClasses) {
                attributesString += ` class="${filteredClasses}"`;
            }
            continue;
        }
        if (attr.value === '') { // Boolean attribute
            attributesString += ` ${attr.name}`;
        } else attributesString += ` ${attr.name}="${attr.value}"`;
    }
    // If the original element had classes but not a class attribute (e.g. added via JS .classList.add)
    // and we haven't added a class attribute yet (e.g. because it wasn't in videoEl.attributes)
    // we should construct it from originalClassList
    if (!videoEl.hasAttribute('class') && !attributesString.includes(' class=')) {
        const filteredClasses = originalClassList.split(' ')
            .filter(cls => !cls.startsWith('vfrag-') && cls !== 'paused' && cls !== '')
            .join(' ');
        if (filteredClasses) {
            attributesString += ` class="${filteredClasses}"`;
        }
    }

    if (activeLeaf.view instanceof MarkdownView) {
        const mdView = activeLeaf.view;
        sourcePathForLink = mdView.file?.path || '';

        if (mdView.getMode() === 'preview' || mdView.getMode() === 'source') {
            const currentVideoSrc = videoEl.currentSrc || videoEl.src; // Prefer live currentSrc for HTML blocks
            if (currentVideoSrc) {
                // Unified resolution for both preview and editor modes
                const resolvedSource = resolveVideoSource(currentVideoSrc, app, sourcePathForLink);
                targetFile = resolvedSource.targetFile;
                isExternalFileUrl = resolvedSource.isExternalFileUrl;
                externalFileUrl = resolvedSource.externalFileUrl;
            }
        } else { // Live Preview mode
            const currentVideoSrc = videoEl.currentSrc || videoEl.src; // Check src directly for HTML blocks in editor
            if (currentVideoSrc) {
                // Unified resolution for both preview and editor modes
                const resolvedSource = resolveVideoSource(currentVideoSrc, app, sourcePathForLink);
                targetFile = resolvedSource.targetFile;
                isExternalFileUrl = resolvedSource.isExternalFileUrl;
                externalFileUrl = resolvedSource.externalFileUrl;
            }
        }
    } else if (activeLeaf.view instanceof FileView && activeLeaf.view.getViewType() === 'video') {
        if (activeLeaf.view.file instanceof TFile) {
            targetFile = activeLeaf.view.file;
            sourcePathForLink = '';
            isExternalFileUrl = false;
        }
    } else return null;

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

    // Clear existing fragment datasets
    delete video.dataset.startTimeRaw;
    delete video.dataset.startTime;
    delete video.dataset.endTimeRaw;
    delete video.dataset.endTime;

    if (fragment) {
        if (fragment.startRaw) video.dataset.startTimeRaw = fragment.startRaw;
        // Only set startTime if it's >= 0 and not 0.001 (remove 0.001 as a placeholder)
        if (fragment.start !== undefined && typeof fragment.start === 'number' && fragment.start >= 0 && fragment.start !== 0.001) video.dataset.startTime = fragment.start.toString();
        if (fragment.endRaw) video.dataset.endTimeRaw = fragment.endRaw;
        if (fragment.end !== undefined && typeof fragment.end === 'number' && fragment.end >= 0) video.dataset.endTime = fragment.end.toString();
    }

    // Update src with new fragment
    const fragmentString = fragment ? generateFragmentString(fragment) : '';
    video.src = `${baseSrc}${fragmentString ? `#${fragmentString}` : ''}`;
}

// Helper for percent object
function isPercentObject(val: any): val is { percent: number } {
    return val && typeof val === 'object' && 'percent' in val && typeof val.percent === 'number';
}

// Helper to compare start and end, supporting percent and number
function compareFragmentTimes(
    start: number | { percent: number } | undefined,
    end: number | { percent: number } | undefined,
    video: HTMLVideoElement
): number | null {
    // Returns -1 if start < end, 0 if equal, 1 if start > end, null if cannot compare
    debug(this, `compareFragmentTimes called with start=${JSON.stringify(start)}, end=${JSON.stringify(end)}`);

    if (start === undefined || end === undefined) return null;

    // Handle infinity specially
    if (typeof end === 'number' && end === Infinity) return -1; // start is always less than infinity
    if (typeof start === 'number' && start === Infinity) return 1; // start infinity is greater than any end

    if (typeof start === 'number' && typeof end === 'number') {
        return start < end ? -1 : (start > end ? 1 : 0);
    }
    if (isPercentObject(start) && isPercentObject(end)) {
        return start.percent < end.percent ? -1 : (start.percent > end.percent ? 1 : 0);
    }
    // Mixed: one is percent, one is number
    const duration = video.duration;
    if (typeof start === 'number' && isPercentObject(end)) {
        if (!duration || !isFinite(duration)) return null;
        const endSec = duration * (end.percent / 100);
        return start < endSec ? -1 : (start > endSec ? 1 : 0);
    }
    if (isPercentObject(start) && typeof end === 'number') {
        if (!duration || !isFinite(duration)) return null;
        const startSec = duration * (start.percent / 100);
        return startSec < end ? -1 : (startSec > end ? 1 : 0);
    }
    return null;
}

export async function setAndSaveVideoFragment(
    app: App,
    video: HTMLVideoElement,
    settings: FragmentsSettings, // Kept for potential future settings-dependent logic
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
            new Notice(`Error updating editor link: ${e.message}`);
            return false;
        }
    } else if (mdView && mdView.editor && !mdView.file) {
        new Notice('Fragment applied to video. Save the file to update the link.');
        return true;
    } else return true;
}

// Helper: update markdown-style video link in the file
async function updateMarkdownLink(
    app: App,
    currentFile: TFile,
    videoInfo: VideoWithFragment,
    subpath: string
) {
    const { line: startLine, col: startCol } = videoInfo.position.start;
    const { line: endLine, col: endCol } = videoInfo.position.end;
    const newLink = generateMarkdownLink({
        app,
        targetPathOrFile: videoInfo.file!,
        sourcePathOrFile: currentFile,
        subpath,
        isEmbed: videoInfo.isEmbedded,
        originalLink: videoInfo.linktext,
        alias: videoInfo.alias
    });
    const content = await app.vault.read(currentFile);
    const lines = content.split('\n');
    if (startLine === endLine) {
        lines[startLine] = lines[startLine].slice(0, startCol) + newLink + lines[startLine].slice(endCol);
    } else {
        const prefix = lines[startLine].slice(0, startCol);
        const suffix = lines[endLine].slice(endCol);
        lines.splice(startLine, endLine - startLine + 1, prefix + newLink + suffix);
    }
    await app.vault.modify(currentFile, lines.join('\n'));
}

// Helper: update HTML-style <video> block in the file
async function updateHtmlLink(
    app: App,
    videoEl: HTMLVideoElement,
    currentFile: TFile,
    videoInfo: VideoWithFragment,
    subpath: string
) {
    const { line: startLine, col: _sc } = videoInfo.position.start;
    const { line: endLine, col: _ec } = videoInfo.position.end;
    const fileContent = await app.vault.read(currentFile);
    const lines = fileContent.split('\n');
    const block = lines.slice(startLine, endLine + 1);
    // locate src line in the block
    let idxInBlock = block.findIndex(l => /<video\s[^>]*src=/i.test(l.trim()));
    if (idxInBlock < 0) throw new Notice('Error: Could not find <video src="..."> in HTML block.');
    const actualLine = startLine + idxInBlock;
    const srcMatch = block[idxInBlock].match(/src=("|')([^"'#]+)/i);
    let base = srcMatch?.[2] ?? (() => {
        const url = new URL(videoEl.dataset.fragmentPath || videoEl.currentSrc || videoEl.src);
        return `${url.protocol}//${url.host}${url.pathname}${url.search}`;
    })();
    const newAttr = `src="${base}${subpath}"`;
    lines[actualLine] = lines[actualLine].replace(/src=("|')[^"'#]+(#[^"']*)?("|')/i, newAttr);
    await app.vault.modify(currentFile, lines.join('\n'));
}

export async function updateEditorLinkInFile(
    app: App,
    videoEl: HTMLVideoElement,
    currentFile: TFile,
    newFragment: TempFragment | null,
    allVideoElementsInView: NodeListOf<HTMLVideoElement>
): Promise<void> {
    const mdView = app.workspace.getActiveViewOfType(MarkdownView)!;
    const allVideosInEditor = markdownExtractor.extract(mdView);
    const idx = Array.from(allVideoElementsInView).indexOf(videoEl);
    const fragmentString = newFragment ? generateFragmentString(newFragment) : '';

    if (idx < 0 || idx >= allVideosInEditor.length) {
        new Notice('Error: Video element not found in editor metadata.');
        return;
    }
    const info = allVideosInEditor[idx];
    const subpath = fragmentString ? `#${fragmentString}` : '';
    try {
        if ((info.type === 'wiki' || info.type === 'md') && info.file) {
            await updateMarkdownLink(app, currentFile, info, subpath);
        } else if (info.type === 'html') {
            await updateHtmlLink(app, videoEl, currentFile, info, subpath);
        } else {
            new Notice('Error: Video type not recognized or file missing.');
        }
    } catch (e: any) {
        new Notice(`Error updating link: ${e.message}`);
    }
}

/**
 * Helper to remove a video embed/link from the note by index.
 * Removes the embed/link at the given index.
 */
export async function removeVideoEmbedByIndex(view: MarkdownView, idx: number): Promise<void> {
    const videos = markdownExtractor.extract(view);
    const els = view.contentEl.querySelectorAll('video');
    if (idx < 0 || idx >= videos.length) return;
    const target = videos[idx];
    const { start, end } = target.position;
    const editor = view.editor;
    const embedText = editor.getRange(
        { line: start.line, ch: start.col },
        { line: end.line, ch: end.col }
    );
    if (/^\s*<video[\s>]/i.test(embedText)) {
        // HTML video tag: remove entire line
        editor.replaceRange(
            '',
            { line: start.line, ch: 0 },
            { line: end.line + 1, ch: 0 }
        );
    } else {
        // Markdown/video embed
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
}

/**
 * Generic helper to copy embed links with optional fragment
 */
export function copyGeneric(
    video: HTMLVideoElement,
    app: App,
    successNotice: string,
    fragment?: string,
    alias?: string,
    fragmentEnd?: string
) {
    const details = getVideoLinkDetails(app, video);
    if (!details) {
        new Notice('Cannot copy link: View type not supported or active leaf not found.');
        return;
    }
    const { targetFile, sourcePathForLink, originalVideoSrcForNotice, isExternalFileUrl, externalFileUrl, attributesString } = details;
    if (!targetFile && !isExternalFileUrl) {
        new Notice(`Video file not found. Source: ${originalVideoSrcForNotice || 'unknown'}`);
        return;
    }
    let linkText: string;
    if (isExternalFileUrl && externalFileUrl) {
        const baseSrc = externalFileUrl.split('#')[0];
        let finalFragment = fragment;
        if (fragmentEnd) {
            finalFragment = `0,${fragmentEnd}`;
        }
        const srcWithFragment = finalFragment ? `${baseSrc}#t=${finalFragment}` : baseSrc;
        linkText = `<video src="${srcWithFragment}"${attributesString}></video>`;
    } else if (targetFile) {
        let finalFragment = fragment;
        if (fragmentEnd) {
            finalFragment = `0,${fragmentEnd}`;
        }
        const subpath = finalFragment ? `#t=${finalFragment}` : undefined;
        linkText = generateMarkdownLink({
            app: app,
            targetPathOrFile: targetFile,
            sourcePathOrFile: sourcePathForLink,
            subpath,
            alias,
            isEmbed: true
        });
    } else {
        new Notice('Could not determine link type.');
        return;
    }
    navigator.clipboard.writeText(linkText)
        .then(() => { new Notice(successNotice); })
        .catch(e => { new Notice(`Failed to copy link to clipboard: ${e instanceof Error ? e.message : String(e)}`); });
}