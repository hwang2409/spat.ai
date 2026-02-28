import { create } from "zustand";
import {
  GameState,
  CaptureStatus,
  DEFAULT_GAME_STATE,
  DEFAULT_CAPTURE_STATUS,
} from "../types";

interface GameStateStore {
  gameState: GameState;
  captureStatus: CaptureStatus;
  setGameState: (state: Partial<GameState>) => void;
  setCaptureStatus: (status: Partial<CaptureStatus>) => void;
}

export const useGameState = create<GameStateStore>((set) => ({
  gameState: DEFAULT_GAME_STATE,
  captureStatus: DEFAULT_CAPTURE_STATUS,
  setGameState: (partial) =>
    set((s) => ({ gameState: { ...s.gameState, ...partial } })),
  setCaptureStatus: (partial) =>
    set((s) => ({ captureStatus: { ...s.captureStatus, ...partial } })),
}));
