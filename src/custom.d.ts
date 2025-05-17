import { VideoState } from './timestamps/types';

declare global {
  interface HTMLVideoElement {
    _timestampState?: VideoState;
  }
}

declare module 'obsidian' {
  interface WorkspaceLeaf {
    _videoTsPatched?: boolean;
    loadIfDeferred(): Promise<void>;
  }
}
