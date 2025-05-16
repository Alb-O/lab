import { TFile, MarkdownView, normalizePath, App, FileSystemAdapter, FileView } from 'obsidian';
import { extractVideosFromMarkdownView } from '../video';

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
                            console.warn(`VideoTimestamps: Vault adapter is not FileSystemAdapter, cannot resolve app:// URL. Converting to file:/// protocol. Original src: ${currentVideoSrc}`);
                            let fileUrlPath = absPathFromUrl; // absPathFromUrl was derived from URL(currentVideoSrc).pathname
                            if (!absPathFromUrl.startsWith('/')) {
                                fileUrlPath = '/' + absPathFromUrl;
                            }
                            externalFileUrl = `file://${fileUrlPath}`;
                            isExternalFileUrl = true;
                            targetFile = null;
                        }
                    } catch (e) {
                        console.error('VideoTimestamps: Error parsing app:// URL for video path:', currentVideoSrc, e);
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
                                }
                                if (!targetFile) { 
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
                            console.warn("VideoTimestamps: Vault adapter is not FileSystemAdapter, cannot resolve app:// URL. Converting to file:/// protocol. Original src: ${currentVideoSrc}");
                            let fileUrlPath = absPathFromUrl;
                            if (!absPathFromUrl.startsWith('/')) {
                                fileUrlPath = '/' + absPathFromUrl;
                            }
                            externalFileUrl = `file://${fileUrlPath}`;
                            isExternalFileUrl = true;
                            targetFile = null;
                        }
                    } catch (e) {
                        console.error('VideoTimestamps: Error parsing app:// URL for video path:', currentVideoSrc, e);
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
