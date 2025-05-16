import { TFile, MarkdownView, normalizePath, App, FileSystemAdapter, FileView } from 'obsidian';
import { extractVideosFromMarkdownView } from '../video';

export interface VideoLinkDetails {
    targetFile: TFile | null;
    sourcePathForLink: string;
    originalVideoSrcForNotice: string | null;
    isExternalFileUrl: boolean;
    externalFileUrl: string | null; // Full src attribute for external file URLs
}

export function getVideoLinkDetails(app: App, videoEl: HTMLVideoElement): VideoLinkDetails | null {
    const activeLeaf = app.workspace.activeLeaf;
    if (!activeLeaf) {
        return null;
    }

    let targetFile: TFile | null = null;
    let sourcePathForLink: string = '';
    const originalVideoSrcForNotice: string | null = videoEl.dataset.timestampPath || videoEl.currentSrc || videoEl.src;
    let isExternalFileUrl = false;
    let externalFileUrl: string | null = null;

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
                                console.warn(`VideoTimestamps: app:// URL path '${absPathFromUrl}' is outside the vault base path '${vaultBasePath}'. Treating as external. Original src: ${currentVideoSrc}`);
                                isExternalFileUrl = true;
                                externalFileUrl = currentVideoSrc; // Store the original app:// URL
                                targetFile = null;
                            }
                        } else {
                            console.warn("VideoTimestamps: Vault adapter is not FileSystemAdapter, cannot resolve app:// URL to vault relative path using getBasePath(). Treating as external.");
                            // If not FileSystemAdapter, we can't determine if it's in vault, so treat as external for safety.
                            isExternalFileUrl = true;
                            externalFileUrl = currentVideoSrc;
                            targetFile = null;
                        }
                    } catch (e) {
                        console.error('VideoTimestamps: Error parsing app:// URL for video path:', currentVideoSrc, e);
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
        } else { // Source or Live Preview mode (less likely to encounter raw file:/// HTML blocks here directly managed by this logic)
            isExternalFileUrl = false; // Assume vault files in editor modes for VideoWithTimestamp
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
    } else if (activeLeaf.view instanceof FileView && activeLeaf.view.getViewType() === 'video') {
        if (activeLeaf.view.file instanceof TFile) {
            targetFile = activeLeaf.view.file;
            sourcePathForLink = ''; 
            isExternalFileUrl = false;
        }
    } else {
        return null;
    }

    return { targetFile, sourcePathForLink, originalVideoSrcForNotice, isExternalFileUrl, externalFileUrl };
}
