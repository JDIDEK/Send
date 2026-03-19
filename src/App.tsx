import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

type TransferProgress = {
  stage: "starting" | "connected" | "progress" | "saving" | "finished" | "error";
  message: string | null;
  bytesDone: number;
  totalBytes: number | null;
  percent: number | null;
  speedBps: number | null;
  etaSeconds: number | null;
};

type Toast = {
  id: number;
  kind: "success" | "error" | "info";
  message: string;
};

function formatBytes(value: number) {
  if (value < 1024) return `${value} o`;
  if (value < 1024 ** 2) return `${(value / 1024).toFixed(1)} Ko`;
  if (value < 1024 ** 3) return `${(value / 1024 ** 2).toFixed(1)} Mo`;
  return `${(value / 1024 ** 3).toFixed(2)} Go`;
}

function formatSpeed(value: number | null) {
  if (!value || value <= 0) return "Calcul...";
  return `${formatBytes(value)}/s`;
}

function formatEta(value: number | null) {
  if (!value || value <= 0) return "Bientot";
  const minutes = Math.floor(value / 60);
  const seconds = value % 60;
  if (minutes === 0) return `${seconds}s restantes`;
  return `${minutes}m ${seconds}s restantes`;
}

function TransferCard({
  title,
  progress,
}: {
  title: string;
  progress: TransferProgress;
}) {
  const progressWidth = `${Math.max(4, Math.min(progress.percent ?? 0, 100))}%`;

  return (
    <div className="w-full space-y-4 p-6 bg-surface-dim animate-in fade-in duration-300">
      <div className="flex items-center justify-between gap-4">
        <h3 className="text-lg font-medium text-ink">{title}</h3>
        <span className="text-sm font-medium text-brand">
          {progress.percent !== null ? `${Math.round(progress.percent)}%` : "En attente"}
        </span>
      </div>

      <div className="h-3 w-full overflow-hidden bg-surface">
        <div
          className="h-full bg-brand transition-[width] duration-300 ease-out"
          style={{ width: progressWidth }}
        />
      </div>

      <div className="grid grid-cols-1 gap-2 text-sm text-ink-muted md:grid-cols-3">
        <p>{progress.message ?? "Transfert en cours"}</p>
        <p>
          {formatBytes(progress.bytesDone)}
          {progress.totalBytes ? ` / ${formatBytes(progress.totalBytes)}` : ""}
        </p>
        <p>{formatSpeed(progress.speedBps)}</p>
      </div>

      <p className="text-xs text-ink-muted">
        {progress.stage === "finished"
          ? "Transfert terminé."
          : progress.stage === "saving"
            ? "Écriture finale sur le disque."
            : formatEta(progress.etaSeconds)}
      </p>
    </div>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = useState<"send" | "receive">("send");
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [fileSize, setFileSize] = useState<string | null>(null);
  const [isSharing, setIsSharing] = useState(false);
  const [ticket, setTicket] = useState<string | null>(null);
  const [uploadProgress, setUploadProgress] = useState<TransferProgress | null>(null);
  const [receiveTicket, setReceiveTicket] = useState("");
  const [isReceiving, setIsReceiving] = useState(false);
  const [receivedPath, setReceivedPath] = useState<string | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<TransferProgress | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);

  const pushToast = (kind: Toast["kind"], message: string) => {
    const id = Date.now() + Math.floor(Math.random() * 1000);
    setToasts((current) => [...current, { id, kind, message }]);

    window.setTimeout(() => {
      setToasts((current) => current.filter((toast) => toast.id !== id));
    }, 4000);
  };

  useEffect(() => {
    let disposed = false;
    let unlistenUpload: undefined | (() => Promise<void> | void);
    let unlistenDownload: undefined | (() => Promise<void> | void);

    void (async () => {
      unlistenUpload = await listen<TransferProgress>("upload-progress", (event) => {
        if (!disposed) {
          setUploadProgress(event.payload);
        }
      });

      unlistenDownload = await listen<TransferProgress>("download-progress", (event) => {
        if (!disposed) {
          setDownloadProgress(event.payload);
        }
      });
    })();

    return () => {
      disposed = true;
      void unlistenUpload?.();
      void unlistenDownload?.();
    };
  }, []);

  const handleSelectFile = async () => {
    try {
      const filePath = await open({ multiple: false, directory: false });

      if (filePath && typeof filePath === "string") {
        setSelectedFile(filePath);
        setTicket(null);
        setUploadProgress(null);

        const size = await invoke<string>("get_file_info", { path: filePath });
        setFileSize(size);
      }
    } catch (error) {
      console.error(error);
      pushToast("error", "Impossible d'ouvrir le sélecteur de fichier.");
    }
  };

  const handleShare = async () => {
    if (!selectedFile) return;

    setIsSharing(true);
    setUploadProgress(null);

    try {
      const generatedTicket = await invoke<string>("share_file", { path: selectedFile });
      setTicket(generatedTicket);
      pushToast("success", "Ticket sécurisé généré.");
    } catch (error) {
      pushToast("error", `Erreur lors de la création du ticket : ${String(error)}`);
    } finally {
      setIsSharing(false);
    }
  };

  const handleReceive = async () => {
    const trimmedTicket = receiveTicket.trim();
    if (!trimmedTicket) return;

    setIsReceiving(true);
    setReceivedPath(null);
    setDownloadProgress(null);

    try {
      const savedPath = await invoke<string>("receive_file", { ticket: trimmedTicket });
      setReceivedPath(savedPath);
      pushToast("success", "Fichier reçu avec succès.");
    } catch (error) {
      pushToast("error", `Téléchargement impossible : ${String(error)}`);
    } finally {
      setIsReceiving(false);
    }
  };

  return (
    <main className="min-h-screen flex flex-col p-8 md:p-12">
      <header className="mb-12 flex justify-between items-baseline">
        <h1 className="text-ink">AltSendme</h1>
        <nav className="flex gap-6 text-lg font-medium">
          <button
            onClick={() => setActiveTab("send")}
            className={`transition-colors duration-200 ease-out ${activeTab === "send" ? "text-brand" : "text-ink-muted hover:text-ink"}`}
          >
            Envoyer
          </button>
          <button
            onClick={() => setActiveTab("receive")}
            className={`transition-colors duration-200 ease-out ${activeTab === "receive" ? "text-brand" : "text-ink-muted hover:text-ink"}`}
          >
            Recevoir
          </button>
        </nav>
      </header>

      <section className="flex-1 flex flex-col justify-center max-w-2xl">
        {activeTab === "send" ? (
          <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500 ease-out">
            <h2>Transférez sans limite.</h2>
            <p className="text-xl text-ink-muted max-w-lg leading-relaxed">
              Sélectionnez un fichier. Il sera envoyé directement de votre ordinateur à celui du destinataire.
            </p>

            <div className="pt-4 flex flex-col items-start gap-6">
              <button
                onClick={handleSelectFile}
                className="bg-ink hover:bg-ink-muted text-surface px-8 py-4 text-lg font-medium cursor-pointer transition-colors duration-200 ease-out"
              >
                Sélectionner un fichier
              </button>

              {selectedFile && !ticket && (
                <div className="w-full space-y-4 animate-in fade-in duration-300">
                  <div className="p-4 bg-surface-dim border-l-4 border-ink">
                    <p className="text-ink font-medium break-all">{selectedFile}</p>
                    <p className="text-ink-muted text-sm mt-1">Taille : {fileSize}</p>
                  </div>

                  <button
                    onClick={handleShare}
                    disabled={isSharing}
                    className="w-full bg-brand hover:bg-brand-hover disabled:bg-surface-dim disabled:text-ink-muted text-surface px-8 py-4 text-lg font-medium cursor-pointer transition-colors duration-200 ease-out flex justify-center items-center"
                  >
                    {isSharing ? "Génération du ticket sécurisé..." : "Générer le lien de transfert"}
                  </button>
                </div>
              )}

              {uploadProgress && (isSharing || uploadProgress.stage !== "finished") && (
                <TransferCard title="Préparation du transfert" progress={uploadProgress} />
              )}

              {ticket && (
                <div className="w-full space-y-4 p-6 bg-surface-dim animate-in zoom-in-95 duration-400 ease-out">
                  <h3 className="text-lg font-medium text-ink">Fichier prêt à être envoyé !</h3>
                  <p className="text-ink-muted">Copiez ce ticket et envoyez-le au destinataire :</p>

                  <div className="flex gap-2">
                    <input
                      readOnly
                      value={ticket}
                      className="flex-1 bg-surface border-none text-ink px-4 py-3 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-brand"
                    />
                    <button
                      onClick={async () => {
                        await navigator.clipboard.writeText(ticket);
                        pushToast("info", "Ticket copié dans le presse-papiers.");
                      }}
                      className="bg-brand hover:bg-brand-hover text-surface px-6 font-medium cursor-pointer transition-colors duration-200"
                    >
                      Copier
                    </button>
                  </div>
                  <p className="text-xs text-brand font-medium mt-4">Ne fermez pas cette application tant que le transfert n'est pas terminé.</p>
                </div>
              )}
            </div>
          </div>
        ) : (
          <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500 ease-out">
            <h2>Recevez instantanément.</h2>
            <p className="text-xl text-ink-muted max-w-lg leading-relaxed">
              Collez le ticket reçu. Le fichier sera téléchargé directement dans votre dossier Téléchargements.
            </p>

            <div className="pt-4 flex flex-col items-start gap-6 w-full">
              <div className="w-full space-y-4">
                <textarea
                  value={receiveTicket}
                  onChange={(event) => {
                    setReceiveTicket(event.target.value);
                    setReceivedPath(null);
                  }}
                  placeholder="Collez ici le ticket sécurisé"
                  className="w-full min-h-32 bg-surface-dim border-none text-ink px-4 py-4 font-mono text-sm leading-relaxed resize-y focus:outline-none focus:ring-2 focus:ring-brand"
                />

                <button
                  onClick={handleReceive}
                  disabled={isReceiving || !receiveTicket.trim()}
                  className="w-full bg-brand hover:bg-brand-hover disabled:bg-surface-dim disabled:text-ink-muted text-surface px-8 py-4 text-lg font-medium cursor-pointer transition-colors duration-200 ease-out flex justify-center items-center"
                >
                  {isReceiving ? "Téléchargement sécurisé en cours..." : "Recevoir le fichier"}
                </button>
              </div>

              {downloadProgress && (isReceiving || downloadProgress.stage !== "finished") && (
                <TransferCard title="Téléchargement sécurisé" progress={downloadProgress} />
              )}

              {receivedPath && (
                <div className="w-full space-y-3 p-6 bg-surface-dim animate-in zoom-in-95 duration-400 ease-out">
                  <h3 className="text-lg font-medium text-ink">Fichier reçu.</h3>
                  <p className="text-ink-muted">Le contenu a été enregistré ici :</p>
                  <div className="p-4 bg-surface border-l-4 border-ink">
                    <p className="text-ink font-medium break-all">{receivedPath}</p>
                  </div>
                  <p className="text-xs text-brand font-medium">
                    Les anciens tickets sans nom de fichier sont enregistrés avec un nom AltSendme générique.
                  </p>
                </div>
              )}
            </div>
          </div>
        )}
      </section>

      <div className="pointer-events-none fixed right-4 bottom-4 z-50 flex w-full max-w-sm flex-col gap-3">
        {toasts.map((toast) => (
          <div
            key={toast.id}
            className={`pointer-events-auto p-4 shadow-sm animate-in slide-in-from-bottom-3 duration-300 ${
              toast.kind === "error"
                ? "bg-ink text-surface"
                : toast.kind === "success"
                  ? "bg-brand text-surface"
                  : "bg-surface-dim text-ink"
            }`}
          >
            <p className="text-sm font-medium">{toast.message}</p>
          </div>
        ))}
      </div>
    </main>
  );
}
