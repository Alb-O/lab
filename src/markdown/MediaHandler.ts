import { MarkdownView, TFile } from "obsidian";
import { splitSubpath, parseLink } from "obsidian-dev-utils/obsidian/Link";
import { TempFragment, parseTempFrag } from "@utils";

/**
 * Represents a video with fragment information found in a markdown document
 */
export interface VideoWithFragment {
    type: 'wiki' | 'md' | 'html';
    file: TFile | null;
    path: string;
    linktext: string;
    alias?: string;
    fragment: TempFragment | null;
    startRaw?: string;
    endRaw?: string;
    isEmbedded: boolean;
    position: { start: { line: number; col: number }; end: { line: number; col: number } };
    originalLinkPath: string;
    originalSubpath: string | null;
}

type Pos = { line: number; col: number };

interface RawMatch<T> {
    rawType: 'wiki' | 'md' | 'html';
    matchData: { [key: string]: any; index: number; input: string; groups?: { [key: string]: string } };
    lineIndex: number;
    handler: MediaHandler<T>;
}

type FileResolverFn = (linkPath: string, sourcePath: string) => TFile | null;

interface MediaHandler<T> {
    findRawMatches(lines: string[]): RawMatch<T>[];
    parseRawMatch(raw: RawMatch<T>, view: MarkdownView, resolve: FileResolverFn, activeFile: TFile): T | null;
}

class VideoHandler implements MediaHandler<VideoWithFragment> {
    private wikiRegex = /(!)?\[\[([^\]\|]+)(?:\|([^\]]+))?\]\]/g;
    private mdRegex = /!\[([^\]]*)\]\(([^)]+)\)|(?<!\!)\[([^\]]+)\]\(([^)]+)\)/g;
    private htmlVideoRegex = /<video[^>]*src\s*=\s*["']([^"'#]+)((?:#[^"']*)?)["'][^>]*>/gi;

    public findRawMatches(lines: string[]): RawMatch<VideoWithFragment>[] {
        const matches: RawMatch<VideoWithFragment>[] = [];
        lines.forEach((line, i) => {
            let m: RegExpExecArray | null;
            this.wikiRegex.lastIndex = 0;
            while ((m = this.wikiRegex.exec(line))) {
                matches.push({ rawType: 'wiki', matchData: { ...m, groups: m.groups }, lineIndex: i, handler: this });
            }
            this.mdRegex.lastIndex = 0;
            while ((m = this.mdRegex.exec(line))) {
                matches.push({ rawType: 'md', matchData: { ...m, groups: m.groups }, lineIndex: i, handler: this });
            }
            this.htmlVideoRegex.lastIndex = 0;
            while ((m = this.htmlVideoRegex.exec(line))) {
                matches.push({ rawType: 'html', matchData: { ...m, groups: m.groups }, lineIndex: i, handler: this });
            }
        });
        return matches;
    }

    public parseRawMatch(raw: RawMatch<VideoWithFragment>, view: MarkdownView, resolve: FileResolverFn, activeFile: TFile): VideoWithFragment | null {
        const { rawType, matchData: m, lineIndex: i } = raw;
        let entry: VideoWithFragment | null = null;
        const srcResolver = (p: string) => resolve(p, activeFile.path);

        if (rawType === 'wiki') {
            const isEmbedded = !!m[1];
            const rawLink = m[2];
            const { linkPath, subpath } = splitSubpath(rawLink);
            const file = srcResolver(linkPath);
            if (file && this.isVideoFile(file)) {
                const pos = { start: { line: i, col: m.index }, end: { line: i, col: m.index + String(m[0]).length } };
                const fragment = subpath && subpath.toLowerCase().startsWith('#t=') ? parseTempFrag(subpath.substring(1)) : null;
                const parsed = parseLink(String(m[0]));
                entry = { type: 'wiki', file, path: file.path, linktext: String(m[0]), alias: parsed?.alias, fragment,
                    startRaw: fragment?.startRaw, endRaw: fragment?.endRaw, isEmbedded, position: pos,
                    originalLinkPath: linkPath, originalSubpath: subpath || null };
            }
        } else if (rawType === 'md') {
            const isEmbedded = m[1] !== undefined;
            let url = isEmbedded ? m[2] : m[4];
            let [pathPart, ...rest] = url.split('#');
            const subpath = rest.length ? `#${rest.join('#')}` : null;
            const file = srcResolver(pathPart);
            if (file && this.isVideoFile(file)) {
                const fragment = subpath && subpath.toLowerCase().startsWith('#t=') ? parseTempFrag(subpath.substring(1)) : null;
                const pos = { start: { line: i, col: m.index }, end: { line: i, col: m.index + m[0].length } };
                const parsed = parseLink(m[0]);
                entry = { type: 'md', file, path: view.app.vault.getResourcePath(file), linktext: m[0], alias: parsed?.alias,
                    fragment, startRaw: fragment?.startRaw, endRaw: fragment?.endRaw, isEmbedded, position: pos,
                    originalLinkPath: pathPart, originalSubpath: subpath };
            }
        } else if (rawType === 'html') {
            const tag = String(m[0]);
            const rawSrc = m[1];
            const frag = m[2] || null;
            let file = srcResolver(rawSrc);
            let videoPath = rawSrc;
            if (file && this.isVideoFile(file)) { videoPath = file.path; }
            const isExternal = /^(https?|file):\/\//i.test(rawSrc);
            if ((file && this.isVideoFile(file)) || isExternal) {
                const pos = { start: { line: i, col: m.index }, end: { line: i, col: m.index + tag.length } };
                const fragment = frag ? parseTempFrag(frag.replace(/^#/, '')) : null;
                entry = { type: 'html', file, path: videoPath, linktext: tag, fragment,
                    startRaw: fragment?.startRaw, endRaw: fragment?.endRaw,
                    isEmbedded: true, position: pos,
                    originalLinkPath: rawSrc, originalSubpath: frag };
            }
        }
        return entry;
    }

    private isVideoFile(file: TFile): boolean {
        const exts = ['mkv','mov','mp4','ogv','webm'];
        return exts.includes(file.extension.toLowerCase());
    }
}

export class LinkExtractor {
    private handlers: MediaHandler<VideoWithFragment>[] = [];
    private fileCache = new Map<string, TFile | null>();

    constructor() {
        this.handlers.push(new VideoHandler());
    }

    public extract(view: MarkdownView): VideoWithFragment[] {
        if (!view || !view.file) return [];
        const activeFile = view.file;
        const lines = view.editor.getValue().split(/\r?\n/);
        // create resolver capturing view
        const resolver: FileResolverFn = (linkPath, sourcePath) => {
            const key = `${linkPath}|${sourcePath}`;
            if (this.fileCache.has(key)) return this.fileCache.get(key)!;
            const file = view.app.metadataCache.getFirstLinkpathDest(linkPath, sourcePath) || null;
            this.fileCache.set(key, file);
            return file;
        };
        const rawMatches = this.handlers.flatMap(h => h.findRawMatches(lines));
        rawMatches.sort((a, b) => a.lineIndex - b.lineIndex || a.matchData.index - b.matchData.index);
        const result: VideoWithFragment[] = [];
        for (const raw of rawMatches) {
            const item = raw.handler.parseRawMatch(raw, view, resolver, activeFile);
            if (item) result.push(item);
        }
        return result;
    }
}

// singleton extractor instance for markdown
const extractor = new LinkExtractor();

/**
 * Shared singleton LinkExtractor — register new handlers here once
 */
export const markdownExtractor = extractor;
