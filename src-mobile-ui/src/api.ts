import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type ModelStatus = {
  downloaded: boolean;
  loaded: boolean;
  bytes: number;
};

export type DownloadProgress = {
  pct: number;
  downloaded: number;
  total: number;
};

export const api = {
  startRecording:        () => invoke<void>("start_recording"),
  stopRecording:         () => invoke<string>("stop_recording"),
  cancelRecording:       () => invoke<void>("cancel_recording"),
  modelStatus:           () => invoke<ModelStatus>("model_status"),
  downloadDefaultModel:  () => invoke<void>("download_default_model"),
};

export function onDownloadProgress(
  cb: (p: DownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen<DownloadProgress>("model/download/progress", (e) => cb(e.payload));
}

export function onModelReady(cb: (name: string) => void): Promise<UnlistenFn> {
  return listen<{ name: string }>("model/ready", (e) => cb(e.payload.name));
}
