import { useEffect, useState, useRef } from 'react';
import { RadarMap, type Position } from './components/RadarMap';

type ViewId = 'home' | 'plants' | 'sys';
type StreamState = 'CONNECTING' | 'LIVE' | 'IDLE' | 'OFFLINE';

const VIEWS: { id: ViewId; label: string; key: string }[] = [
  { id: 'home', label: 'HOME', key: '1' },
  { id: 'plants', label: 'PLANTS', key: '2' },
  { id: 'sys', label: 'SYS', key: '3' },
];

export default function App() {
  const [activeView, setActiveView] = useState<ViewId>('home');
  const [targets, setTargets] = useState<Position[]>([]);
  const [streamState, setStreamState] = useState<StreamState>('CONNECTING');
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // SSE Telemetry Stream
  useEffect(() => {
    const sse = new EventSource('/api/map/stream');

    sse.onopen = () => setStreamState('IDLE');
    sse.onerror = () => setStreamState('OFFLINE');

    sse.onmessage = (event) => {
      try {
        const data: Position[] = JSON.parse(event.data);
        setTargets(data);
        setStreamState('LIVE');

        if (timeoutRef.current) clearTimeout(timeoutRef.current);
        timeoutRef.current = setTimeout(() => {
          setTargets([]);
          setStreamState('IDLE');
        }, 2000);
      } catch (e) {
        console.error('Failed to parse telemetry:', e);
      }
    };

    return () => {
      sse.close();
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, []);

  // Keyboard shortcuts (1, 2, 3) for quick view switching
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const match = VIEWS.find((v) => v.key === e.key);
      if (match) setActiveView(match.id);
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  const statusColor = {
    LIVE: 'bg-red-500 shadow-[0_0_8px_#ef4444]',
    IDLE: 'bg-amber-400/80',
    CONNECTING: 'bg-zinc-500 animate-pulse',
    OFFLINE: 'bg-zinc-700',
  }[streamState];

  return (
    <div className="flex h-screen w-screen flex-col bg-[#030305] text-zinc-300 font-mono select-none overflow-hidden p-4 gap-3">
      
      {/* Top Navigation & Telemetry Bar */}
      <header className="flex h-10 shrink-0 items-center justify-between rounded border border-zinc-800/80 bg-[#08080c] px-4 text-xs">
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-2 tracking-widest text-red-500 font-semibold">
            <span className={`h-2 w-2 rounded-full ${statusColor}`} />
            <span>STARK</span>
          </div>

          <nav className="flex items-center gap-1">
            {VIEWS.map((v) => {
              const active = activeView === v.id;
              return (
                <button
                  key={v.id}
                  onClick={() => setActiveView(v.id)}
                  className={`px-2.5 py-1 rounded text-[11px] tracking-wider transition-colors cursor-pointer ${
                    active
                      ? 'bg-red-500/15 text-red-400 border border-red-500/40'
                      : 'text-zinc-500 hover:text-zinc-300 border border-transparent'
                  }`}
                >
                  <span className="opacity-40 mr-1">{v.key}</span>
                  {v.label}
                </button>
              );
            })}
          </nav>
        </div>

        <div className="flex items-center gap-4 text-[11px] tracking-widest text-zinc-500">
          <span>
            TRK <strong className="text-zinc-200">{targets.length}</strong>
          </span>
          <span className="text-zinc-700">|</span>
          <span className={streamState === 'OFFLINE' ? 'text-red-500/70' : 'text-zinc-400'}>
            {streamState}
          </span>
        </div>
      </header>

      {/* Dedicated Viewport Container */}
      <main className="relative flex-1 min-h-0 w-full rounded-lg border border-zinc-800/80 bg-[#050508] shadow-[inset_0_0_80px_rgba(0,0,0,0.9)] flex items-center justify-center p-6 overflow-hidden">
        
        {/* Subtle CRT Scanline Overlay across the entire viewport */}
        <div className="absolute inset-0 z-20 pointer-events-none bg-[linear-gradient(transparent_50%,rgba(0,0,0,0.35)_50%)] bg-[length:100%_4px] opacity-50" />

        {activeView === 'home' && <RadarMap targets={targets} />}

        {activeView === 'plants' && (
          <div className="text-xs tracking-widest text-zinc-600">
            [ PLANTS MODULE STANDBY ]
          </div>
        )}

        {activeView === 'sys' && (
          <div className="text-xs tracking-widest text-zinc-600">
            [ SYSTEM DIAGNOSTICS STANDBY ]
          </div>
        )}
      </main>
    </div>
  );
}