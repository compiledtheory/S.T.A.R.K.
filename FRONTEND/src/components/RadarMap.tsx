import { TargetMarker } from './TargetMarker';

export interface Position {
  id?: string | number;
  x: number;
  y: number;
}

interface RadarMapProps {
  targets: Position[];
}

interface Zone {
  id: string;
  label: string;
  x: number;
  y: number;
  w: number;
  h: number;
  color: string;
  vertical?: boolean;
}

const SVG_WIDTH = 9520;
const SVG_HEIGHT = 4000;

const ZONES: Zone[] = [
  { id: 'hallway',  label: 'HALLWAY',  x: 20,   y: 20,   w: 5000, h: 1480, color: '#ffd3a2' },
  { id: 'bedroom',  label: 'BEDROOM',  x: 20,   y: 1500, w: 3200, h: 2500, color: '#f8a2ff' },
  { id: 'bathroom', label: 'BATHROOM', x: 3220, y: 1500, w: 1800, h: 2500, color: '#a2d9ff' },
  { id: 'living',   label: 'LIVING',   x: 5020, y: 20,   w: 4480, h: 3980, color: '#eaffb1' },
  { id: 'kitchen',  label: 'KITCHEN',  x: 5031, y: 1500, w: 1000, h: 2500, color: '#ffb1c2', vertical: true },
  { id: 'desk',     label: 'DESK',     x: 6245, y: 20,   w: 1400, h: 1000, color: '#655dff' },
  { id: 'couch',    label: 'COUCH',    x: 6556, y: 1503, w: 2000, h: 1400, color: '#b1ffbe' },
];

function getZoneForPoint(nx: number, ny: number): string {
  const px = nx * SVG_WIDTH;
  const py = ny * SVG_HEIGHT;
  for (let i = ZONES.length - 1; i >= 0; i--) {
    const z = ZONES[i];
    if (px >= z.x && px <= z.x + z.w && py >= z.y && py <= z.y + z.h) {
      return z.label;
    }
  }
  return 'EXT';
}

export function RadarMap({ targets }: RadarMapProps) {
  const normalizedTargets = targets.map((t) => ({
    ...t,
    x: t.x > 1 ? t.x / SVG_WIDTH : t.x,
    y: t.y > 1 ? t.y / SVG_HEIGHT : t.y,
  }));

  return (
    <div className="relative w-full max-w-6xl max-h-full aspect-[9520/4000] font-mono select-none">
      <svg
        viewBox={`0 0 ${SVG_WIDTH} ${SVG_HEIGHT}`}
        className="block h-full w-full pointer-events-none"
      >
        <defs>
          <pattern id="tactical-grid" width="500" height="500" patternUnits="userSpaceOnUse">
            <path
              d="M 500 0 L 0 0 0 500"
              fill="none"
              stroke="rgba(255,255,255,0.03)"
              strokeWidth="8"
            />
          </pattern>
        </defs>

        <rect width={SVG_WIDTH} height={SVG_HEIGHT} fill="url(#tactical-grid)" />

        {ZONES.map((z) => {
          const cx = z.x + z.w / 2;
          const cy = z.y + z.h / 2;
          const isOccupied = normalizedTargets.some((t) => {
            const px = t.x * SVG_WIDTH;
            const py = t.y * SVG_HEIGHT;
            return px >= z.x && px <= z.x + z.w && py >= z.y && py <= z.y + z.h;
          });

          return (
            <g key={z.id}>
              <rect
                x={z.x}
                y={z.y}
                width={z.w}
                height={z.h}
                fill={z.color}
                fillOpacity={isOccupied ? 0.16 : 0.07}
                stroke={z.color}
                strokeOpacity={isOccupied ? 0.65 : 0.25}
                strokeWidth="12"
                className="transition-all duration-300"
              />
              {z.label !== 'LIVING' && (
                <text
                  x={cx}
                  y={cy}
                  fill={z.color}
                  fillOpacity={isOccupied ? 0.95 : 0.55}
                  fontSize="220"
                  fontFamily="monospace"
                  fontWeight="600"
                  letterSpacing="0.15em"
                  textAnchor="middle"
                  dominantBaseline="central"
                  transform={z.vertical ? `rotate(-90 ${cx} ${cy})` : undefined}
                  className="transition-opacity duration-300"
                >
                  {z.label}
                </text>
              )}
            </g>
          );
        })}

        {/* Outer Perimeter Wall */}
        <rect
          x="10"
          y="10"
          width="9500"
          height="3980"
          fill="none"
          stroke="rgba(255,255,255,0.35)"
          strokeWidth="20"
        />

        {/* Interior Structural Walls */}
        <g fill="rgba(255,255,255,0.75)">
          <rect x="3210" y="1500" width="20" height="2500" />
          <rect x="5011" y="1500" width="20" height="2500" />
          <rect x="20" y="1490" width="3824" height="20" />
          <rect x="4433" y="1490" width="600" height="20" />
        </g>
      </svg>

      {normalizedTargets.map((target, idx) => (
        <TargetMarker
          key={target.id ?? idx}
          index={idx + 1}
          x={target.x}
          y={target.y}
          zone={getZoneForPoint(target.x, target.y)}
        />
      ))}
    </div>
  );
}