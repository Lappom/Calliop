interface AudioLevelBarsProps {
  level: number;
}

const BAR_SCALES = [0.35, 0.6, 1, 0.6, 0.35];

export function AudioLevelBars({ level }: AudioLevelBarsProps) {
  const clamped = Math.min(1, Math.max(0, level));

  return (
    <div
      className="flex h-8 items-end gap-1"
      aria-hidden
    >
      {BAR_SCALES.map((scale, index) => (
        <div
          key={index}
          className="w-1 rounded-full bg-accent-green transition-[height] duration-75 ease-out"
          style={{
            height: `${Math.max(4, clamped * scale * 28)}px`,
          }}
        />
      ))}
    </div>
  );
}
