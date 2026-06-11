import type { ReactNode } from "react";
import type { GlowColor } from "../layout/glowSurface";
import { glowSurfaceClasses } from "../layout/glowSurface";
import type { SettingsSectionId } from "./settingsUtils";
import { settingsSectionDomId } from "./settingsUtils";

interface SettingsSectionProps {
  id: SettingsSectionId;
  title: string;
  description: string;
  glow?: GlowColor;
  children: ReactNode;
}

export function SettingsSection({
  id,
  title,
  description,
  glow = "blue",
  children,
}: SettingsSectionProps) {
  return (
    <section
      id={settingsSectionDomId(id)}
      aria-labelledby={`${settingsSectionDomId(id)}-title`}
    >
      <header className="mb-4">
        <h2
          id={`${settingsSectionDomId(id)}-title`}
          className="text-heading-sm m-0 text-ink"
        >
          {title}
        </h2>
        <p className="text-body-sm mt-2 text-charcoal">{description}</p>
      </header>
      <div
        className={[
          glowSurfaceClasses(glow),
          "rounded-lg border border-hairline-strong bg-surface-card p-5 sm:p-6",
        ].join(" ")}
      >
        <div className="relative space-y-6">{children}</div>
      </div>
    </section>
  );
}
