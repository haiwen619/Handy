import { useState } from "react";
import LogPanel from "./components/LogPanel";
import ModelDownloadGate from "./components/ModelDownloadGate";
import RecordButton from "./components/RecordButton";
import TranscriptDisplay from "./components/TranscriptDisplay";

export default function App() {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);

  return (
    <ModelDownloadGate>
      <main className="flex h-full flex-col gap-4 p-4">
        <header className="text-center text-xl font-semibold">Handy</header>

        <div className="flex flex-col items-center gap-4">
          <RecordButton onTranscript={setText} onError={setError} />

          {error && (
            <p className="rounded bg-red-900/40 px-3 py-2 text-sm text-red-200">
              {error}
            </p>
          )}

          <TranscriptDisplay text={text} />
        </div>

        <div className="mt-auto">
          <LogPanel />
        </div>
      </main>
    </ModelDownloadGate>
  );
}
