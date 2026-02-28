import { useGameState } from "../../hooks";

const COST_COLORS: Record<number, string> = {
  1: "border-gray-400 text-gray-300",
  2: "border-green-500 text-green-400",
  3: "border-blue-500 text-blue-400",
  4: "border-purple-500 text-purple-400",
  5: "border-yellow-500 text-yellow-400",
};

export function ShopPanel() {
  const shop = useGameState((s) => s.gameState.shop);

  const hasChampions = shop.some((s) => s.champion !== null);

  return (
    <div className="rounded-lg bg-tft-panel p-4">
      <h2 className="mb-3 text-sm font-semibold uppercase tracking-wider text-tft-gold">
        Shop
      </h2>

      {!hasChampions ? (
        <p className="text-sm text-gray-500">
          Waiting for shop recognition...
        </p>
      ) : (
        <div className="grid grid-cols-5 gap-2">
          {shop.map((slot) => (
            <ShopSlotCard key={slot.index} slot={slot} />
          ))}
        </div>
      )}
    </div>
  );
}

function ShopSlotCard({
  slot,
}: {
  slot: { index: number; champion: { id: string; name: string; cost: number; traits: string[] } | null; confidence: number };
}) {
  const { champion, confidence } = slot;

  if (!champion) {
    return (
      <div className="flex h-16 items-center justify-center rounded border border-gray-700 bg-gray-800/50 text-xs text-gray-600">
        Empty
      </div>
    );
  }

  const costStyle = COST_COLORS[champion.cost] ?? "border-gray-600 text-gray-400";
  const confidencePct = Math.round(confidence * 100);

  return (
    <div
      className={`flex h-16 flex-col items-center justify-center rounded border-2 bg-gray-800/80 px-1 ${costStyle}`}
    >
      <span className="truncate text-xs font-medium leading-tight">
        {champion.name}
      </span>
      <span className="mt-0.5 text-[10px] opacity-60">
        {champion.cost}g
      </span>
      <span className="text-[9px] opacity-40">{confidencePct}%</span>
    </div>
  );
}
