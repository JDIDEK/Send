import { useState } from "react";

import { ReceivePanel } from "./components/ReceivePanel";
import { SendPanel } from "./components/SendPanel";
import { ToastViewport } from "./components/ToastViewport";
import { useTransfer } from "./hooks/useTransfer";

export default function App() {
  const [activeTab, setActiveTab] = useState<"send" | "receive">("send");
  const { sendPanel, receivePanel, toasts } = useTransfer();

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
        {activeTab === "send" ? <SendPanel {...sendPanel} /> : <ReceivePanel {...receivePanel} />}
      </section>

      <ToastViewport toasts={toasts} />
    </main>
  );
}
