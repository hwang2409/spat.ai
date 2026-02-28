export type ShopAction = "buy" | "skip" | "consider";

export interface ShopAdvice {
  slotIndex: number;
  action: ShopAction;
  reason: string;
}

export type EconAction = "level" | "roll" | "save" | "slow-roll";

export interface EconAdvice {
  action: EconAction;
  reason: string;
  targetGold?: number;
}

export interface ItemAdvice {
  item1: string;
  item2: string;
  result: string;
  priority: number;
  reason: string;
}

export interface CompAdvice {
  compName: string;
  matchScore: number;
  missingUnits: string[];
  coreItems: string[];
}

export interface Advice {
  shop: ShopAdvice[];
  econ: EconAdvice | null;
  items: ItemAdvice[];
  comp: CompAdvice | null;
  llmInsight: string | null;
  timestamp: number;
}

export const DEFAULT_ADVICE: Advice = {
  shop: [],
  econ: null,
  items: [],
  comp: null,
  llmInsight: null,
  timestamp: 0,
};
