import type { TransferStatusViewModel } from "../utils/transferFormatting";
import { TransferStatusCard } from "./TransferStatusCard";

type ReceivePanelProps = {
  receiveTicket: string;
  isReceiving: boolean;
  receivedPath: string | null;
  progress: TransferStatusViewModel | null;
  onTicketChange: (value: string) => void;
  onReceive: () => Promise<void>;
};

export function ReceivePanel({
  receiveTicket,
  isReceiving,
  receivedPath,
  progress,
  onTicketChange,
  onReceive,
}: ReceivePanelProps) {
  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500 ease-out">
      <h2>Recevez instantanément.</h2>
      <p className="text-xl text-ink-muted max-w-lg leading-relaxed">
        Collez le ticket reçu. Le fichier sera téléchargé directement dans votre dossier Téléchargements.
      </p>

      <div className="pt-4 flex flex-col items-start gap-6 w-full">
        <div className="w-full space-y-4">
          <textarea
            value={receiveTicket}
            onChange={(event) => onTicketChange(event.target.value)}
            placeholder="Collez ici le ticket sécurisé"
            className="w-full min-h-32 bg-surface-dim border-none text-ink px-4 py-4 font-mono text-sm leading-relaxed resize-y focus:outline-none focus:ring-2 focus:ring-brand"
          />

          <button
            onClick={() => void onReceive()}
            disabled={isReceiving || !receiveTicket.trim()}
            className="w-full bg-brand hover:bg-brand-hover disabled:bg-surface-dim disabled:text-ink-muted text-surface px-8 py-4 text-lg font-medium cursor-pointer transition-colors duration-200 ease-out flex justify-center items-center"
          >
            {isReceiving ? "Téléchargement sécurisé en cours..." : "Recevoir le fichier"}
          </button>
        </div>

        {progress && <TransferStatusCard {...progress} />}

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
  );
}
