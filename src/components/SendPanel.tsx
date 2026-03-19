import type { TransferStatusViewModel } from "../utils/transferFormatting";
import { TransferStatusCard } from "./TransferStatusCard";

type SendPanelProps = {
  selectedFile: string | null;
  fileSize: string | null;
  isSharing: boolean;
  ticket: string | null;
  progress: TransferStatusViewModel | null;
  onSelectFile: () => Promise<void>;
  onShare: () => Promise<void>;
  onCopyTicket: () => Promise<void>;
};

export function SendPanel({
  selectedFile,
  fileSize,
  isSharing,
  ticket,
  progress,
  onSelectFile,
  onShare,
  onCopyTicket,
}: SendPanelProps) {
  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500 ease-out">
      <h2>Transférez sans limite.</h2>
      <p className="text-xl text-ink-muted max-w-lg leading-relaxed">
        Sélectionnez un fichier. Il sera envoyé directement de votre ordinateur à celui du destinataire.
      </p>

      <div className="pt-4 flex flex-col items-start gap-6">
        <button
          onClick={() => void onSelectFile()}
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
              onClick={() => void onShare()}
              disabled={isSharing}
              className="w-full bg-brand hover:bg-brand-hover disabled:bg-surface-dim disabled:text-ink-muted text-surface px-8 py-4 text-lg font-medium cursor-pointer transition-colors duration-200 ease-out flex justify-center items-center"
            >
              {isSharing ? "Génération du ticket sécurisé..." : "Générer le lien de transfert"}
            </button>
          </div>
        )}

        {progress && <TransferStatusCard {...progress} />}

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
                onClick={() => void onCopyTicket()}
                className="bg-brand hover:bg-brand-hover text-surface px-6 font-medium cursor-pointer transition-colors duration-200"
              >
                Copier
              </button>
            </div>
            <p className="text-xs text-brand font-medium mt-4">
              Ne fermez pas cette application tant que le transfert n'est pas terminé.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
