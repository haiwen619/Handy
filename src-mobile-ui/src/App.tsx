import { useState } from "react";
import ModelDownloadGate from "./components/ModelDownloadGate";
import RecordButton from "./components/RecordButton";
import TranscriptDisplay from "./components/TranscriptDisplay";

export default function App() {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);

  return (
    <ModelDownloadGate>
      <main className="flex h-full flex-col items-center justify-between gap-6 p-6">
        <header className="text-xl font-semibold">Handy</header>

        <RecordButton onTranscript={setText} onError={setError} />

        {error && (
          <p className="rounded bg-red-900/40 px-3 py-2 text-sm text-red-200">
            {error}
          </p>
        )}

        <TranscriptDisplay text={text} />
      </main>
    </ModelDownloadGate>
  );
}
