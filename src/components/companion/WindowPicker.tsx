import { useCallback, useEffect, useState } from "react";

interface WindowInfo {
  title: string;
  app_name: string;
  width: number;
  height: number;
}

export function WindowPicker() {
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [selectedTitle, setSelectedTitle] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const refreshWindows = useCallback(async () => {
    setLoading(true);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke<WindowInfo[]>("list_windows");
      setWindows(result);
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

      {selectedTitle && (
        <div className="mb-2 flex items-center gap-2">
          <span className="text-xs text-green-400">Targeting:</span>
          <span className="truncate text-xs text-gray-300">{selectedTitle}</span>
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
          <p className="text-xs text-gray-500">No windows found</p>
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
