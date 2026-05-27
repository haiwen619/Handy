import { writeText } from "@tauri-apps/plugin-clipboard-manager";

type Props = { text: string };

export default function TranscriptDisplay({ text }: Props) {
  const onCopy = async () => {
    if (!text) return;
    await writeText(text);
  };

  return (
    <div className="flex w-full flex-col gap-3">
      <div className="min-h-24 rounded-md border border-neutral-700 bg-neutral-900 p-3 text-left">
        {text || <span className="text-neutral-500">Transcript will appear here…</span>}
      </div>
      <button
        onClick={onCopy}
        disabled={!text}
        className="rounded-md bg-neutral-700 px-4 py-2 disabled:opacity-40"
      >
        Copy
      </button>
    </div>
  );
}
