import type { ReactNode } from "react";
import type { SettingsSectionId } from "./settingsUtils";
import { settingsSectionDomId } from "./settingsUtils";

interface SettingsSectionProps {
  id: SettingsSectionId;
  title: string;
  description: string;
  children: ReactNode;
}

export function SettingsSection({
  id,
  title,
  description,
  children,
}: SettingsSectionProps) {
  return (
    <section
      id={settingsSectionDomId(id)}
      aria-labelledby={`${settingsSectionDomId(id)}-title`}
    >
      <header className="mb-8">
        <h2
          id={`${settingsSectionDomId(id)}-title`}
          className="text-heading-md m-0 text-ink"
        >
          {title}
        </h2>
        <p className="text-body-sm mt-2 text-charcoal">{description}</p>
      </header>
      <div className="space-y-6">{children}</div>
    </section>
  );
}
