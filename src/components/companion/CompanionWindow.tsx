import { useEffect } from "react";
import { useCaptureEvents } from "../../hooks";
import { CaptureStatusPanel } from "./CaptureStatusPanel";
import { WindowPicker } from "./WindowPicker";
import { ShopPanel } from "./ShopPanel";
import { EconomyPanel } from "./EconomyPanel";

export function CompanionWindow() {
  useCaptureEvents();

  useEffect(() => {
    async function startCapture() {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("start_capture");
      } catch {
        // Not running in Tauri or command not available yet
      }
    }
    startCapture();
  }, []);

  return (
    <div className="flex min-h-screen flex-col bg-tft-dark p-4">
      <header className="mb-6">
        <h1 className="text-2xl font-bold text-tft-gold">spat.ai</h1>
        <p className="text-sm text-gray-400">Real-time game assistant</p>
      </header>

      <div className="grid gap-4">
        <CaptureStatusPanel />
        <WindowPicker />
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
