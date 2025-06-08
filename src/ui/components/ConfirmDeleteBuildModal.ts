import { App, Modal } from "obsidian";
import { BlenderBuildInfo } from "@/types";
import { debug, info, warn, error, registerLoggerClass } from '@/utils/obsidian-logger';

export class ConfirmDeleteBuildModal extends Modal {
	private onAccept: () => void;
	private build: BlenderBuildInfo;	constructor(app: App, build: BlenderBuildInfo, onAccept: () => void) {
		super(app);
		registerLoggerClass(this, 'ConfirmDeleteBuildModal');
		debug('Initializing ConfirmDeleteBuildModal', {
			buildSubversion: build?.subversion,
			buildBranch: build?.branch,
			hasCustomExecutable: !!build?.customExecutable
		});
		
		this.build = build;
		this.onAccept = onAccept;
	}

	onOpen() {
		const { contentEl, modalEl } = this;
		contentEl.empty();
		modalEl.addClass('mod-blender-delete-build-confirm');
		
		const modalHeader = modalEl.querySelector('.modal-header');
		if (modalHeader) {
			modalHeader.createDiv('modal-title', el => {
				el.textContent = 'Delete Blender build?';
			});
		}

		contentEl.createEl("p", { 
			text: `Are you sure you want to delete Blender ${this.build.subversion}? This will remove both the downloaded archive and extracted files. This action cannot be undone.` 
		});

		const buttonRow = contentEl.createDiv('modal-button-container');
		
		// Delete button (left)
		const deleteBtn = buttonRow.createEl('button', { text: 'Delete build' });
		deleteBtn.addClass('mod-warning');
		deleteBtn.onclick = () => {
			this.onAccept();
			this.close();
		};
		
		// Cancel button (right)
		const cancelBtn = buttonRow.createEl('button', { text: 'Cancel' });
		cancelBtn.onclick = () => this.close();
	}
}
