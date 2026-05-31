import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type LogLevel = "info" | "warn" | "error";

export type LogLine = {
  level: LogLevel;
  msg: string;
  ts_ms: number;
};

const MAX_LINES = 200;

const levelColor: Record<LogLevel, string> = {
  info: "text-slate-300",
  warn: "text-amber-300",
  error: "text-red-400",
};

function fmtTime(ts: number): string {
  const d = new Date(ts);
  const pad = (n: number, w = 2) => String(n).padStart(w, "0");
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}.${pad(d.getMilliseconds(), 3)}`;
}

/**
 * Tail of recent backend log lines, shown at the bottom of the screen.
 * Listens to the Rust backend's `app/log` event. Useful on Android where
 * `adb logcat` is the only other way to see what the backend is doing —
 * and adb requires a tethered debug session. Toggle with the header button.
 */
export default function LogPanel() {
  const [open, setOpen] = useState(true);
  const [lines, setLines] = useState<LogLine[]>([]);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;
    listen<LogLine>("app/log", (e) => {
      setLines((prev) => {
        const next = prev.concat(e.payload);
        return next.length > MAX_LINES ? next.slice(next.length - MAX_LINES) : next;
      });
    }).then((u) => {
      if (cancelled) u();
      else unlisten = u;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    // Auto-scroll to bottom whenever new lines arrive.
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [lines]);

  return (
    <section className="flex w-full flex-col rounded border border-slate-700 bg-slate-900/80 text-xs">
      <header className="flex items-center justify-between px-3 py-1.5">
        <span className="font-semibold text-slate-200">
          Logs ({lines.length})
        </span>
        <div className="flex gap-2">
          <button
            type="button"
            className="rounded bg-slate-700 px-2 py-0.5 text-slate-100 hover:bg-slate-600"
            onClick={() => setLines([])}
          >
            Clear
          </button>
          <button
            type="button"
            className="rounded bg-slate-700 px-2 py-0.5 text-slate-100 hover:bg-slate-600"
            onClick={() => setOpen((v) => !v)}
          >
            {open ? "Hide" : "Show"}
          </button>
        </div>
      </header>
      {open && (
        <div
          ref={scrollRef}
          className="max-h-48 overflow-y-auto border-t border-slate-700 px-3 py-2 font-mono leading-tight"
        >
          {lines.length === 0 ? (
            <p className="text-slate-500">(no events yet)</p>
          ) : (
            lines.map((l, i) => (
              <div key={i} className={levelColor[l.level] ?? "text-slate-300"}>
                <span className="text-slate-500">{fmtTime(l.ts_ms)}</span>{" "}
                <span className="uppercase">{l.level}</span> {l.msg}
              </div>
            ))
          )}
        </div>
      )}
    </section>
  );
}
