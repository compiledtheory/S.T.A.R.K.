interface TargetMarkerProps {
  index: number;
  x: number;
  y: number;
  zone: string;
}

export function TargetMarker({ index, x, y, zone }: TargetMarkerProps) {
  return (
    <div
      className="absolute z-30 -translate-x-1/2 -translate-y-1/2 transition-all duration-300 ease-linear font-mono"
      style={{
        left: `${x * 100}%`,
        top: `${y * 100}%`,
      }}
    >
      <div className="absolute -inset-2.5 rounded-full border border-red-500/60 animate-ping pointer-events-none" />

      <div className="relative flex h-4 w-4 items-center justify-center rounded-full border border-red-400/80 bg-red-950/60 shadow-[0_0_16px_rgba(239,68,68,0.9)]">
        <div className="h-1.5 w-1.5 rounded-full bg-red-500" />
      </div>

      <div className="absolute left-3 top-1/2 -translate-y-1/2 flex items-center gap-1.5 rounded bg-black/85 px-1.5 py-0.5 text-[10px] leading-none border border-red-500/30 whitespace-nowrap shadow-md">
        <span className="font-bold text-red-400">#{index}</span>
        <span className="text-zinc-300">{zone}</span>
      </div>
    </div>
  );
}