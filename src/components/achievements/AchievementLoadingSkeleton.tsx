import { glowSurfaceClasses } from "../layout/glowSurface";

function SkeletonBlock({ className = "" }: { className?: string }) {
  return (
    <div
      className={[
        "animate-pulse rounded-md bg-surface-elevated",
        className,
      ].join(" ")}
      aria-hidden
    />
  );
}

export function AchievementLoadingSkeleton() {
  return (
    <div className="flex flex-col gap-8" aria-busy="true" aria-label="Loading">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="space-y-3">
          <SkeletonBlock className="h-10 w-48" />
          <SkeletonBlock className="h-4 w-72" />
        </div>
        <SkeletonBlock className="size-10 rounded-md" />
      </div>

      <div
        className={[
          glowSurfaceClasses("green"),
          "rounded-lg border border-hairline-strong bg-surface-card p-6 sm:p-8",
        ].join(" ")}
      >
        <div className="relative flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
          <div className="flex-1 space-y-3">
            <SkeletonBlock className="h-3 w-24" />
            <SkeletonBlock className="h-12 w-40" />
            <SkeletonBlock className="h-4 w-56" />
            <SkeletonBlock className="mt-2 h-2 w-full max-w-md rounded-full" />
          </div>
          <div className="flex flex-wrap gap-3">
            {Array.from({ length: 4 }).map((_, i) => (
              <SkeletonBlock
                key={i}
                className="h-[72px] w-full min-w-[120px] sm:w-[140px]"
              />
            ))}
          </div>
        </div>
      </div>

      <div className="flex flex-wrap gap-2">
        {Array.from({ length: 7 }).map((_, i) => (
          <SkeletonBlock key={i} className="h-9 w-28 rounded-full" />
        ))}
      </div>

      <div className="flex flex-wrap gap-2">
        {Array.from({ length: 4 }).map((_, i) => (
          <SkeletonBlock key={i} className="h-8 w-24 rounded-full" />
        ))}
      </div>

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
        {Array.from({ length: 6 }).map((_, i) => (
          <SkeletonBlock key={i} className="h-44 rounded-lg" />
        ))}
      </div>
    </div>
  );
}
