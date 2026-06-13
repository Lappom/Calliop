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

export function InsightLoadingSkeleton() {
  return (
    <div className="flex flex-col gap-8" aria-busy="true" aria-label="Loading">
      <div
        className={[
          glowSurfaceClasses("blue"),
          "rounded-lg border border-hairline-strong bg-surface-card p-6 sm:p-8",
        ].join(" ")}
      >
        <div className="relative flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
          <div className="flex-1 space-y-3">
            <SkeletonBlock className="h-3 w-24" />
            <SkeletonBlock className="h-12 w-40" />
            <SkeletonBlock className="h-4 w-56" />
          </div>
          <div className="flex flex-wrap gap-3">
            <SkeletonBlock className="h-[88px] w-full min-w-[140px] sm:w-[180px]" />
            <SkeletonBlock className="h-[88px] w-full min-w-[140px] sm:w-[180px]" />
          </div>
        </div>
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <SkeletonBlock className="h-36 rounded-lg" />
        <SkeletonBlock className="h-36 rounded-lg" />
      </div>

      <div className="flex flex-wrap gap-2">
        {Array.from({ length: 4 }).map((_, i) => (
          <SkeletonBlock key={i} className="h-9 w-24 rounded-full" />
        ))}
      </div>

      <div className="space-y-4">
        <SkeletonBlock className="h-3 w-20" />
        <SkeletonBlock className="h-[360px] w-full rounded-lg" />
        <SkeletonBlock className="h-20 w-full rounded-lg" />
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <SkeletonBlock className="h-[360px] w-full rounded-lg" />
        <SkeletonBlock className="h-[360px] w-full rounded-lg" />
      </div>

      <div className="space-y-4">
        <SkeletonBlock className="h-3 w-16" />
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <SkeletonBlock key={i} className="h-20 rounded-lg" />
          ))}
        </div>
        <SkeletonBlock className="h-[320px] w-full rounded-lg" />
      </div>
    </div>
  );
}
