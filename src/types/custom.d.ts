import { VideoState } from '@types';

declare global {
  interface HTMLVideoElement {
    _fragmentState?: VideoState;
    _videoContextMenuHandler?: (event: MouseEvent) => void;
    _metadataHandler?: (event: Event) => void;
    _fragmentMasterHandler?: (event: Event) => void;
    _justResetFromEnd?: boolean;
    _seekedToEnd?: boolean;
    _seekedToEndTimeout?: ReturnType<typeof setTimeout>;
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
