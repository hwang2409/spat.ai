export interface Champion {
  id: string;
  name: string;
  cost: number;
  traits: string[];
}

export interface ShopSlot {
  index: number;
  champion: Champion | null;
  confidence: number;
}

export interface ItemSlot {
  id: string;
  name: string;
  isComponent: boolean;
}

export interface BoardSlot {
  row: number;
  col: number;
  champion: Champion | null;
  starLevel: number;
  items: ItemSlot[];
}

export interface GameState {
  gold: number;
  level: number;
  stage: string;
  shop: ShopSlot[];
  bench: BoardSlot[];
  board: BoardSlot[];
  items: ItemSlot[];
  round: number;
}

export interface CaptureStatus {
  isCapturing: boolean;
  windowFound: boolean;
  windowTitle: string | null;
  fps: number;
  lastCaptureTime: number | null;
  resolution: [number, number] | null;
}

export const DEFAULT_GAME_STATE: GameState = {
  gold: 0,
  level: 1,
  stage: "1-1",
  shop: [],
  bench: [],
  board: [],
  items: [],
  round: 0,
};

export const DEFAULT_CAPTURE_STATUS: CaptureStatus = {
  isCapturing: false,
  windowFound: false,
  windowTitle: null,
  fps: 0,
  lastCaptureTime: null,
  resolution: null,
};
