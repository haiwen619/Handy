import { useEffect, useState } from "react";
import { api, onDownloadProgress, onModelReady, type DownloadProgress } from "../api";

type GateProps = { children: React.ReactNode };

export default function ModelDownloadGate({ children }: GateProps) {
  const [status, setStatus] = useState<"checking" | "downloading" | "ready">("checking");
  const [progress, setProgress] = useState<DownloadProgress | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let unlistenProgress: (() => void) | undefined;
    let unlistenReady: (() => void) | undefined;

    (async () => {
      try {
        const s = await api.modelStatus();
        if (s.downloaded) {
          setStatus("ready");
          return;
        }
        setStatus("downloading");
        unlistenProgress = await onDownloadProgress(setProgress);
        unlistenReady = await onModelReady(() => setStatus("ready"));
        await api.downloadDefaultModel();
      } catch (e: any) {
        setError(String(e));
      }
    })();

    return () => {
      unlistenProgress?.();
      unlistenReady?.();
    };
  }, []);

  if (status === "ready") return <>{children}</>;

  return (
    <div className="flex h-full flex-col items-center justify-center p-8 text-center">
      <h2 className="mb-4 text-xl font-semibold">First-time setup</h2>
      {error ? (
        <p className="text-red-400">Error: {error}</p>
      ) : status === "checking" ? (
        <p>Checking model…</p>
      ) : (
        <>
          <p className="mb-2">Downloading speech model (~39 MB)…</p>
          <div className="h-2 w-64 overflow-hidden rounded bg-neutral-800">
            <div
              className="h-full bg-emerald-500 transition-all"
              style={{ width: `${progress?.pct ?? 0}%` }}
            />
          </div>
          <p className="mt-2 text-sm text-neutral-400">{progress?.pct ?? 0}%</p>
        </>
      )}
    </div>
  );
}
