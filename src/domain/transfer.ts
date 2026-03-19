export type TransferStage =
  | "starting"
  | "connected"
  | "progress"
  | "saving"
  | "finished"
  | "error";

export type ToastKind = "success" | "error" | "info";

export type Unsubscribe = () => Promise<void> | void;

export interface TransferProgress {
  stage: TransferStage;
  message: string | null;
  bytesDone: number;
  totalBytes: number | null;
  percent: number | null;
  speedBps: number | null;
  etaSeconds: number | null;
}

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
}

export interface TransferClient {
  selectFile(): Promise<string | null>;
  getFileInfo(path: string): Promise<string>;
  shareFile(path: string): Promise<string>;
  receiveFile(ticket: string): Promise<string>;
  copyToClipboard(value: string): Promise<void>;
  subscribeUploadProgress(handler: (progress: TransferProgress) => void): Promise<Unsubscribe>;
  subscribeDownloadProgress(handler: (progress: TransferProgress) => void): Promise<Unsubscribe>;
}
