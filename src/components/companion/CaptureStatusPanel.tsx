import { useState } from "react";
import { useGameState } from "../../hooks";

export function CaptureStatusPanel() {
  const captureStatus = useGameState((s) => s.captureStatus);
  const [debugPath, setDebugPath] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const saveDebugFrame = async () => {
    setSaving(true);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const path = await invoke<string | null>("save_debug_frame");
      setDebugPath(path);
    } catch {
      // Not in Tauri
    }
    setSaving(false);
  };

  const statusColor = captureStatus.isCapturing
    ? "bg-green-500"
    : captureStatus.windowFound
      ? "bg-yellow-500"
      : "bg-red-500";

  const statusText = captureStatus.isCapturing
    ? "Capturing"
    : captureStatus.windowFound
      ? "Window Found"
      : "No window selected";

  return (
    <div className="rounded-lg bg-tft-panel p-4">
      <h2 className="mb-3 text-sm font-semibold uppercase tracking-wider text-tft-gold">
        Capture Status
      </h2>

      <div className="flex items-center gap-3">
        <div className={`h-3 w-3 rounded-full ${statusColor} animate-pulse`} />
        <span className="text-sm">{statusText}</span>
      </div>

      {captureStatus.windowTitle && (
        <div className="mt-2 text-xs text-gray-400">
          Window: {captureStatus.windowTitle}
        </div>
      )}

      {captureStatus.resolution && (
        <div className="mt-1 text-xs text-gray-400">
          Resolution: {captureStatus.resolution[0]}x{captureStatus.resolution[1]}
        </div>
      )}

      {captureStatus.isCapturing && (
        <div className="mt-1 text-xs text-gray-400">
          FPS: {captureStatus.fps.toFixed(1)}
        </div>
      )}

      {captureStatus.lastCaptureTime && (
        <div className="mt-1 text-xs text-gray-400">
          Last capture:{" "}
          {new Date(captureStatus.lastCaptureTime).toLocaleTimeString()}
        </div>
      )}

      {captureStatus.isCapturing && (
        <div className="mt-3 border-t border-gray-700 pt-3">
          <button
            onClick={saveDebugFrame}
            disabled={saving}
            className="rounded bg-tft-accent px-3 py-1.5 text-xs text-gray-300 hover:bg-tft-accent/80 disabled:opacity-50"
          >
            {saving ? "Saving..." : "Save Debug Frame"}
          </button>
          {debugPath && (
            <div className="mt-1 text-[10px] text-gray-500 break-all">
              Saved to: {debugPath}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
