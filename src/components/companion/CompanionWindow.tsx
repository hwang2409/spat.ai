import { useState } from "react";
import { useCaptureEvents } from "../../hooks";
import { CaptureStatusPanel } from "./CaptureStatusPanel";
import { ShopPanel } from "./ShopPanel";
import { EconomyPanel } from "./EconomyPanel";

export function CompanionWindow() {
  useCaptureEvents();
  const [loading, setLoading] = useState(false);

  async function handleLoadVideo() {
    try {
      setLoading(true);
      const { open } = await import("@tauri-apps/plugin-dialog");
      const file = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: "Video",
            extensions: ["mp4", "mkv", "mov", "webm", "avi"],
          },
        ],
      });

      if (file) {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("start_video_analysis", { path: file });
      }
    } catch (e) {
      console.error("Failed to load video:", e);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="flex min-h-screen flex-col bg-tft-dark p-4">
      <header className="mb-6 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-tft-gold">spat.ai</h1>
          <p className="text-sm text-gray-400">Real-time game assistant</p>
        </div>
        <button
          onClick={handleLoadVideo}
          disabled={loading}
          className="rounded-lg bg-tft-panel px-4 py-2 text-sm font-medium text-tft-gold transition-colors hover:bg-gray-700 disabled:opacity-50"
        >
          {loading ? "Loading..." : "Load Video"}
        </button>
      </header>

      <div className="grid gap-4">
        <CaptureStatusPanel />
        <EconomyPanel />
        <ShopPanel />

        <div className="rounded-lg bg-tft-panel p-4">
          <h2 className="mb-3 text-sm font-semibold uppercase tracking-wider text-tft-gold">
            Advice
          </h2>
          <p className="text-sm text-gray-500">
            Advice will appear here during gameplay...
          </p>
        </div>
      </div>
    </div>
  );
}
