import { BarChart3 } from "lucide-react";
import { glowSurfaceClasses } from "../layout/glowSurface";

export function InsightEmptyState() {
  return (
    <div
      className={[
        glowSurfaceClasses("blue"),
        "rounded-lg border border-hairline-strong bg-surface-card p-6 sm:p-8",
      ].join(" ")}
    >
      <div className="relative flex items-start gap-4">
        <span className="inline-flex size-10 shrink-0 items-center justify-center rounded-lg border border-hairline-strong bg-surface-elevated text-charcoal">
          <BarChart3 size={18} strokeWidth={1.75} aria-hidden />
        </span>
        <div>
          <p className="text-body-md m-0 text-ink">
            Pas encore de statistiques
          </p>
          <p className="text-body-sm mt-2 text-charcoal">
            Effectuez vos premières dictées pour voir l&apos;activité, la latence
            du pipeline, la répartition par application et votre vitesse de
            parole.
          </p>
        </div>
      </div>
    </div>
  );
}
