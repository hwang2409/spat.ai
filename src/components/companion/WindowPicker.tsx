import { useCallback, useEffect, useState } from "react";

interface WindowInfo {
  title: string;
  app_name: string;
  width: number;
  height: number;
}

interface WindowListResult {
  windows: WindowInfo[];
  raw_count: number;
  has_permission: boolean;
}

export function WindowPicker() {
  const [result, setResult] = useState<WindowListResult | null>(null);
  const [selectedTitle, setSelectedTitle] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const refreshWindows = useCallback(async () => {
    setLoading(true);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const res = await invoke<WindowListResult>("list_windows");
      setResult(res);
    } catch {
      // Not in Tauri
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    refreshWindows();
  }, [refreshWindows]);

  const selectWindow = async (title: string | null) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("set_target_window", { title });
      setSelectedTitle(title);
    } catch {
      // Not in Tauri
    }
  };

  const windows = result?.windows ?? [];
  const hasPermission = result?.has_permission ?? true;

  return (
    <div className="rounded-lg bg-tft-panel p-4">
      <div className="mb-3 flex items-center justify-between">
        <h2 className="text-sm font-semibold uppercase tracking-wider text-tft-gold">
          Window Target
        </h2>
        <button
          onClick={refreshWindows}
          disabled={loading}
          className="rounded bg-tft-accent px-2 py-1 text-xs text-gray-300 hover:bg-tft-accent/80 disabled:opacity-50"
        >
          {loading ? "..." : "Refresh"}
        </button>
      </div>

      {!hasPermission && (
        <div className="mb-3 rounded border border-yellow-600/40 bg-yellow-900/20 p-3">
          <p className="text-xs font-medium text-yellow-400">
            Screen Recording permission required
          </p>
          <p className="mt-1 text-[11px] text-yellow-400/70">
            Go to System Settings &rarr; Privacy &amp; Security &rarr; Screen
            Recording and enable it for this app. Then restart the app.
          </p>
          {result && (
            <p className="mt-1 text-[10px] text-gray-500">
              (Seeing {result.raw_count} raw window{result.raw_count !== 1 && "s"}, {windows.length} usable)
            </p>
          )}
        </div>
      )}

      {selectedTitle && (
        <div className="mb-2 flex items-center gap-2">
          <span className="text-xs text-green-400">Targeting:</span>
          <span className="truncate text-xs text-gray-300">
            {selectedTitle}
          </span>
          <button
            onClick={() => selectWindow(null)}
            className="text-xs text-gray-500 hover:text-gray-300"
          >
            (auto)
          </button>
        </div>
      )}

      <div className="max-h-48 space-y-1 overflow-y-auto">
        {windows.length === 0 ? (
          <p className="text-xs text-gray-500">
            {hasPermission
              ? "No windows found"
              : "Grant Screen Recording permission to see windows"}
          </p>
        ) : (
          windows.map((win, i) => (
            <button
              key={i}
              onClick={() => selectWindow(win.title)}
              className={`block w-full rounded px-2 py-1.5 text-left text-xs transition-colors ${
                selectedTitle === win.title
                  ? "bg-tft-accent text-white"
                  : "bg-gray-800/40 text-gray-400 hover:bg-gray-800/80 hover:text-gray-200"
              }`}
            >
              <div className="truncate font-medium">{win.title}</div>
              <div className="text-[10px] opacity-60">
                {win.app_name} — {win.width}x{win.height}
              </div>
            </button>
          ))
        )}
      </div>
    </div>
  );
}
