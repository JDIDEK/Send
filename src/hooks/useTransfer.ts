import { startTransition, useEffect, useState } from "react";

import type { Toast, TransferClient, TransferProgress } from "../domain/transfer";
import { tauriTransferClient } from "../infrastructure/tauriTransferClient";
import { toTransferStatusViewModel, type TransferStatusViewModel } from "../utils/transferFormatting";
import { useToastQueue } from "./useToastQueue";

type SendPanelState = {
  selectedFile: string | null;
  fileSize: string | null;
  isSharing: boolean;
  ticket: string | null;
  progress: TransferStatusViewModel | null;
  onSelectFile: () => Promise<void>;
  onShare: () => Promise<void>;
  onCopyTicket: () => Promise<void>;
};

type ReceivePanelState = {
  receiveTicket: string;
  isReceiving: boolean;
  receivedPath: string | null;
  progress: TransferStatusViewModel | null;
  onTicketChange: (value: string) => void;
  onReceive: () => Promise<void>;
};

function isVisibleProgress(progress: TransferProgress | null, isBusy: boolean) {
  return Boolean(progress && (isBusy || progress.stage !== "finished"));
}

export function useTransfer(client: TransferClient = tauriTransferClient): {
  sendPanel: SendPanelState;
  receivePanel: ReceivePanelState;
  toasts: Toast[];
} {
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [fileSize, setFileSize] = useState<string | null>(null);
  const [isSharing, setIsSharing] = useState(false);
  const [ticket, setTicket] = useState<string | null>(null);
  const [uploadProgress, setUploadProgress] = useState<TransferProgress | null>(null);
  const [receiveTicket, setReceiveTicket] = useState("");
  const [isReceiving, setIsReceiving] = useState(false);
  const [receivedPath, setReceivedPath] = useState<string | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<TransferProgress | null>(null);
  const { toasts, pushToast } = useToastQueue();

  useEffect(() => {
    let disposed = false;
    let unsubscribeUpload: undefined | (() => Promise<void> | void);
    let unsubscribeDownload: undefined | (() => Promise<void> | void);

    void (async () => {
      try {
        unsubscribeUpload = await client.subscribeUploadProgress((progress) => {
          if (!disposed) {
            startTransition(() => {
              setUploadProgress(progress);
            });
          }
        });

        unsubscribeDownload = await client.subscribeDownloadProgress((progress) => {
          if (!disposed) {
            startTransition(() => {
              setDownloadProgress(progress);
            });
          }
        });
      } catch (error) {
        console.error("Impossible d'initialiser les événements Tauri:", error);
      }
    })();

    return () => {
      disposed = true;
      void unsubscribeUpload?.();
      void unsubscribeDownload?.();
    };
  }, [client]);

  const handleSelectFile = async () => {
    try {
      const filePath = await client.selectFile();
      if (!filePath) {
        return;
      }

      setSelectedFile(filePath);
      setTicket(null);
      setUploadProgress(null);

      const size = await client.getFileInfo(filePath);
      setFileSize(size);
    } catch (error) {
      console.error(error);
      pushToast("error", "Impossible d'ouvrir le sélecteur de fichier.");
    }
  };

  const handleShare = async () => {
    if (!selectedFile) {
      return;
    }

    setIsSharing(true);
    setUploadProgress(null);

    try {
      const generatedTicket = await client.shareFile(selectedFile);
      setTicket(generatedTicket);
      pushToast("success", "Ticket sécurisé généré.");
    } catch (error) {
      pushToast("error", `Erreur lors de la création du ticket : ${String(error)}`);
    } finally {
      setIsSharing(false);
    }
  };

  const handleCopyTicket = async () => {
    if (!ticket) {
      return;
    }

    await client.copyToClipboard(ticket);
    pushToast("info", "Ticket copié dans le presse-papiers.");
  };

  const handleReceive = async () => {
    const trimmedTicket = receiveTicket.trim();
    if (!trimmedTicket) {
      return;
    }

    const destinationPath = await client.chooseReceiveLocation();
    if (!destinationPath) {
      return;
    }

    setIsReceiving(true);
    setReceivedPath(null);
    setDownloadProgress(null);

    try {
      const savedPath = await client.receiveFile(trimmedTicket, destinationPath);
      setReceivedPath(savedPath);
      pushToast("success", "Fichier reçu avec succès.");
    } catch (error) {
      pushToast("error", `Téléchargement impossible : ${String(error)}`);
    } finally {
      setIsReceiving(false);
    }
  };

  const sendProgress = isVisibleProgress(uploadProgress, isSharing)
    ? toTransferStatusViewModel("Préparation du transfert", uploadProgress!)
    : null;
  const receiveProgress = isVisibleProgress(downloadProgress, isReceiving)
    ? toTransferStatusViewModel("Téléchargement sécurisé", downloadProgress!)
    : null;

  return {
    sendPanel: {
      selectedFile,
      fileSize,
      isSharing,
      ticket,
      progress: sendProgress,
      onSelectFile: handleSelectFile,
      onShare: handleShare,
      onCopyTicket: handleCopyTicket,
    },
    receivePanel: {
      receiveTicket,
      isReceiving,
      receivedPath,
      progress: receiveProgress,
      onTicketChange: (value) => {
        setReceiveTicket(value);
        setReceivedPath(null);
      },
      onReceive: handleReceive,
    },
    toasts,
  };
}
