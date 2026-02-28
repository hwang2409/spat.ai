import { useEffect } from "react";
import { useGameState } from "./useGameState";
import type { CaptureStatus, ShopSlot } from "../types";

interface GameStatePayload {
  shop: Array<{
    index: number;
    championId: string | null;
    championName: string | null;
    cost: number | null;
    confidence: number;
  }>;
  gold: number | null;
  level: number | null;
  stage: string | null;
}

export function useCaptureEvents() {
  const setCaptureStatus = useGameState((s) => s.setCaptureStatus);
  const setGameState = useGameState((s) => s.setGameState);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    async function setup() {
      try {
        const { listen } = await import("@tauri-apps/api/event");

        const u1 = await listen<CaptureStatus>("capture-status", (event) => {
          setCaptureStatus(event.payload);
        });
        unlisteners.push(u1);

        const u2 = await listen<GameStatePayload>("game-state", (event) => {
          const payload = event.payload;

          const shop: ShopSlot[] = payload.shop.map((s) => ({
            index: s.index,
            champion: s.championId
              ? {
                  id: s.championId,
                  name: s.championName ?? s.championId,
                  cost: s.cost ?? 0,
                  traits: [],
                }
              : null,
            confidence: s.confidence,
          }));

          setGameState({
            shop,
            ...(payload.gold != null ? { gold: payload.gold } : {}),
            ...(payload.level != null ? { level: payload.level } : {}),
            ...(payload.stage != null ? { stage: payload.stage } : {}),
          });
        });
        unlisteners.push(u2);
      } catch {
        // Not running in Tauri
      }
    }

    setup();

    return () => {
      unlisteners.forEach((u) => u());
    };
  }, [setCaptureStatus, setGameState]);
}
