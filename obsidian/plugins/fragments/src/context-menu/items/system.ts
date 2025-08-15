import { Menu, Plugin, Notice, TAbstractFile, Modal, App, MarkdownView } from 'obsidian';
import { removeVideoEmbedByIndex, getVideoLinkDetails } from '@utils';

export function addSystemCommands(menu: Menu, plugin: Plugin, video: HTMLVideoElement) {
    menu.addItem(item => item
        .setIcon('arrow-up-right')
        .setTitle('Open with default app')
        .setSection('vfrag-system')
        .onClick(async () => {
            const linkDetails = getVideoLinkDetails(plugin.app, video);
            this.app.openWithDefaultApp(linkDetails?.targetFile?.path);
        })
    );

    menu.addItem(item => item
        .setIcon('arrow-up-right')
        .setTitle('Show in system explorer')
        .setSection('vfrag-system')
        .onClick(async () => {
            const linkDetails = getVideoLinkDetails(plugin.app, video);
            this.app.showInFolder(linkDetails?.targetFile?.path);
        })
    );

    menu.addItem(item => item
        .setIcon('folder-open')
        .setTitle('Reveal file in navigation')
        .setSection('vfrag-system')
        .onClick(async () => {
            const linkDetails = getVideoLinkDetails(plugin.app, video);
            this.app.internalPlugins.getEnabledPluginById("file-explorer")?.revealInFolder(linkDetails?.targetFile);
        })
    );

    menu.addItem(item => item
        .setIcon('edit-3')
        .setTitle('Rename...')
        .setSection('vfrag-danger')
        .onClick(async () => {
            const linkDetails = getVideoLinkDetails(plugin.app, video);
            if (!linkDetails || !(linkDetails.targetFile instanceof TAbstractFile)) {
                return new Notice(`Video file not found. Source: ${linkDetails?.targetFile?.path || 'unknown'}`);
            }
            const openRenameFileModal = (app: App, currentName: string, onSubmit: (newName: string) => void) => {
                let newName: string;
                let originalExtension: string;
                const lastDotIndex = currentName.lastIndexOf('.');
                if (lastDotIndex !== -1 && lastDotIndex > 0) { // Ensure dot is not the first character
                    originalExtension = currentName.substring(lastDotIndex);
                    newName = currentName.substring(0, lastDotIndex);
                } else {
                    originalExtension = ''; // No extension or hidden file
                    newName = currentName;
                }
                const modal = new Modal(app);
                let inputEl: HTMLTextAreaElement;
                modal.onOpen = () => {
                    const { contentEl, modalEl } = modal;
                    modalEl.addClass('mod-file-rename');
                    const modalHeader = modalEl.querySelector('.modal-header');
                    modalHeader?.createDiv('modal-title', el => {
                        el.textContent = 'File name';
                    });
                    inputEl = contentEl.createEl('textarea', {
                        cls: 'rename-textarea vfrag-textarea',
                        attr: { rows: '1' }
                    });
                    inputEl.value = newName;
                    inputEl.select();
                    inputEl.addEventListener('keydown', (e) => {
                        if (e.key === 'Enter') {
                            e.preventDefault();
                            submit();
                        }
                    });
                    const buttonContainer = contentEl.createDiv({ cls: 'modal-button-container' });
                    const okBtn = buttonContainer.createEl('button', { text: 'Save', cls: 'mod-cta' });
                    okBtn.onclick = () => submit();
                    const cancelBtn = buttonContainer.createEl('button', { text: 'Cancel' });
                    cancelBtn.onclick = () => modal.close();
                };
                const submit = () => {
                    const value = inputEl.value.trim();
                    // Validation for invalid characters
                    if (/[\\/*?"<>:|]/.test(value)) {
                        new Notice('File name cannot contain any of the following characters: * " \\ / < > : | ?');
                        return modal.close();
                    }
                    if (value) {
                        onSubmit(value + originalExtension); // Re-append extension
                        modal.close();
                    }
                };
                modal.open();
            };
            openRenameFileModal(plugin.app, linkDetails.targetFile.name, (newName) => {
                if (linkDetails.targetFile) {
                    plugin.app.fileManager.renameFile(linkDetails.targetFile, newName).then(() => {
                        new Notice('Successfully renamed file');
                    });
                }
            });
        })
    );

    menu.addItem(item => {
        // @ts-ignore
        item.dom.addClass('is-warning');
        item
        .setIcon('trash')
        .setTitle('Delete video file')
        .setSection('vfrag-danger');
        item.onClick(async () => {
        const view = plugin.app.workspace.getActiveViewOfType(MarkdownView);
        if (!view) {
            new Notice('Deleting video files only works from a Markdown note.');
            return;
        } if (view.getMode() === 'preview') {
            new Notice('Cannot delete while in reading view.');
            return;
        }
        const els = view.contentEl.querySelectorAll('video');
        const idx = Array.from(els).indexOf(video);
        await removeVideoEmbedByIndex(view, idx);
        // Use getVideoLinkDetails to determine the file to remove
        const linkDetails = getVideoLinkDetails(plugin.app, video);
        const fileToDelete = linkDetails?.targetFile;
        if (!fileToDelete) {
            new Notice('Could not determine video file to delete.');
            return;
        } else {
            plugin.app.fileManager.trashFile(fileToDelete);
        }
        });
    });
}