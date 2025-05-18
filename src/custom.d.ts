import { VideoState } from './fragments/types';

declare global {
  interface HTMLVideoElement {
    _fragmentState?: VideoState;
  }
  interface CustomVideoElement extends HTMLVideoElement {
    _shadowStyle?: HTMLStyleElement;
    _debugOverlay?: HTMLElement;
    _fullscreenChangeHandler?: () => void;
  }
  interface CustomDocument extends Document {
    webkitFullscreenElement?: Element;
  }
}

declare module 'obsidian' {
  interface WorkspaceLeaf {
    _videoFrPatched?: boolean;
    loadIfDeferred(): Promise<void>;
  }
}
