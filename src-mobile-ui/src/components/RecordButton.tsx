import { useState } from "react";
import { api } from "../api";

type Props = {
  onTranscript: (text: string) => void;
  onError: (msg: string) => void;
};

export default function RecordButton({ onTranscript, onError }: Props) {
  const [recording, setRecording] = useState(false);
  const [transcribing, setTranscribing] = useState(false);

  const handleDown = async () => {
    try {
      await api.startRecording();
      setRecording(true);
    } catch (e: any) {
      onError(String(e));
    }
  };

  const handleUp = async () => {
    if (!recording) return;
    setRecording(false);
    setTranscribing(true);
    try {
      const text = await api.stopRecording();
      onTranscript(text);
    } catch (e: any) {
      onError(String(e));
    } finally {
      setTranscribing(false);
    }
  };

  return (
    <button
      onPointerDown={handleDown}
      onPointerUp={handleUp}
      onPointerCancel={handleUp}
      onPointerLeave={() => recording && handleUp()}
      disabled={transcribing}
      className={[
        "flex h-40 w-40 select-none items-center justify-center rounded-full text-lg font-semibold transition",
        recording ? "scale-105 bg-red-600 shadow-lg shadow-red-600/40" : "bg-emerald-600",
        transcribing && "opacity-50",
      ].filter(Boolean).join(" ")}
    >
      {transcribing ? "Transcribing…" : recording ? "Release" : "Hold to talk"}
    </button>
  );
}
