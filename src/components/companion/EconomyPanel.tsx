import { useGameState } from "../../hooks";

export function EconomyPanel() {
  const { gold, level, stage } = useGameState((s) => s.gameState);

  return (
    <div className="rounded-lg bg-tft-panel p-4">
      <h2 className="mb-3 text-sm font-semibold uppercase tracking-wider text-tft-gold">
        Economy
      </h2>

      <div className="grid grid-cols-3 gap-3">
        <StatBox label="Gold" value={gold > 0 ? `${gold}` : "—"} color="text-yellow-400" />
        <StatBox label="Level" value={level > 0 ? `${level}` : "—"} color="text-blue-400" />
        <StatBox label="Stage" value={stage || "—"} color="text-gray-300" />
      </div>

      {gold > 0 && (
        <div className="mt-3 text-xs text-gray-400">
          Interest: +{Math.min(Math.floor(gold / 10), 5)}g
          {gold >= 50 && (
            <span className="ml-2 text-green-400">(max interest)</span>
          )}
        </div>
      )}
    </div>
  );
}

function StatBox({
  label,
  value,
  color,
}: {
  label: string;
  value: string;
  color: string;
}) {
  return (
    <div className="rounded bg-gray-800/60 p-2 text-center">
      <div className="text-[10px] uppercase tracking-wider text-gray-500">
        {label}
      </div>
      <div className={`text-lg font-bold ${color}`}>{value}</div>
    </div>
  );
}
