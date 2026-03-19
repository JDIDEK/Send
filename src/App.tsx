import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export default function App() {
  const [activeTab, setActiveTab] = useState<"send" | "receive">("send");
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [fileSize, setFileSize] = useState<string | null>(null);

  const [isSharing, setIsSharing] = useState(false);
  const [ticket, setTicket] = useState<string | null>(null);
  const [receiveTicket, setReceiveTicket] = useState("");
  const [isReceiving, setIsReceiving] = useState(false);
  const [receivedPath, setReceivedPath] = useState<string | null>(null);
  const [receiveError, setReceiveError] = useState<string | null>(null);

  const handleSelectFile = async () => {
    try {
      const filePath = await open({ multiple: false, directory: false });

      if (filePath && typeof filePath === 'string') {
        setSelectedFile(filePath);
        setTicket(null);
        
        const size = await invoke<string>("get_file_info", { path: filePath });
        setFileSize(size);
      }
    } catch (error) {
      console.error(error);
    }
  };

  const handleShare = async () => {
    if (!selectedFile) return;

    setIsSharing(true);
    try {
      const generatedTicket = await invoke<string>("share_file", { path: selectedFile });
      setTicket(generatedTicket);
    } catch (error) {
      alert("Erreur lors de la création du ticket : " + error);
    } finally {
      setIsSharing(false);
    }
  };

  const handleReceive = async () => {
    const trimmedTicket = receiveTicket.trim();
    if (!trimmedTicket) return;

    setIsReceiving(true);
    setReceiveError(null);
    setReceivedPath(null);

    try {
      const savedPath = await invoke<string>("receive_file", { ticket: trimmedTicket });
      setReceivedPath(savedPath);
    } catch (error) {
      setReceiveError(String(error));
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
                      onClick={() => navigator.clipboard.writeText(ticket)}
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
                    setReceiveError(null);
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

              {receiveError && (
                <div className="w-full p-4 bg-surface-dim border-l-4 border-brand">
                  <p className="text-ink font-medium">Le téléchargement a échoué.</p>
                  <p className="text-ink-muted text-sm mt-1 break-words">{receiveError}</p>
                </div>
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
    </main>
  );
}
