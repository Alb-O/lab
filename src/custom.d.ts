import { VideoState } from './fragments/types';

declare global {
  interface HTMLVideoElement {
    _fragmentState?: VideoState;
  }
}

declare module 'obsidian' {
  interface WorkspaceLeaf {
    _videoFrPatched?: boolean;
    loadIfDeferred(): Promise<void>;
  }
}
