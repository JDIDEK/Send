import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

import type { TransferClient, TransferProgress } from "../domain/transfer";

export const tauriTransferClient: TransferClient = {
  async selectFile() {
    const filePath = await open({ multiple: false, directory: false });
    return typeof filePath === "string" ? filePath : null;
  },

  async getFileInfo(path) {
    return invoke<string>("get_file_info", { path });
  },

  async shareFile(path) {
    return invoke<string>("share_file", { path });
  },

  async receiveFile(ticket) {
    return invoke<string>("receive_file", { ticket });
  },

  async copyToClipboard(value) {
    await navigator.clipboard.writeText(value);
  },

  async subscribeUploadProgress(handler) {
    return listen<TransferProgress>("upload-progress", (event) => {
      handler(event.payload);
    });
  },

  async subscribeDownloadProgress(handler) {
    return listen<TransferProgress>("download-progress", (event) => {
      handler(event.payload);
    });
  },
};
