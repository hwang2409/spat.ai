import { create } from "zustand";
import { Advice, DEFAULT_ADVICE } from "../types";

interface AdviceStore {
  advice: Advice;
  setAdvice: (advice: Partial<Advice>) => void;
}

export const useAdvice = create<AdviceStore>((set) => ({
  advice: DEFAULT_ADVICE,
  setAdvice: (partial) =>
    set((s) => ({ advice: { ...s.advice, ...partial } })),
}));
