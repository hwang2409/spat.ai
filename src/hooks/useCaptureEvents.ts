import { useEffect } from "react";
import { useGameState } from "./useGameState";
import type { CaptureStatus } from "../types";

export function useCaptureEvents() {
  const setCaptureStatus = useGameState((s) => s.setCaptureStatus);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    async function setup() {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const unlistenFn = await listen<CaptureStatus>(
          "capture-status",
          (event) => {
            setCaptureStatus(event.payload);
          },
        );
        unlisten = unlistenFn;
      } catch {
        // Not running in Tauri
      }
    }

    setup();

    return () => {
      unlisten?.();
    };
  }, [setCaptureStatus]);
}
