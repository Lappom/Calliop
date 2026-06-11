import type { ReactNode } from "react";
import { Mic, MousePointerClick, ToggleLeft } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Kbd } from "../ui/Kbd";

const ALT_MARKER = "%ALT%";
const SPACE_MARKER = "%SPACE%";

function HotkeyToggleDescription() {
  const { t } = useTranslation();
  const template = t("keys.toggleDescription", {
    alt: ALT_MARKER,
    space: SPACE_MARKER,
  });
  const segments = template.split(
    new RegExp(`(${ALT_MARKER}|${SPACE_MARKER})`),
  );

  return (
    <>
      {segments.map((segment, index) => {
        if (segment === ALT_MARKER) {
          return <Kbd key={index}>{t("keys.modifiers.alt")}</Kbd>;
        }
        if (segment === SPACE_MARKER) {
          return <Kbd key={index}>{t("keys.space")}</Kbd>;
        }
        return segment;
      })}
    </>
  );
}

export function MainHotkeyGuide() {
  const { t } = useTranslation();

  return (
    <div className="grid gap-3 sm:grid-cols-3">
      <HintCard
        icon={<Mic size={16} strokeWidth={1.75} aria-hidden />}
        title={t("keys.toggleTitle")}
        description={<HotkeyToggleDescription />}
      />
      <HintCard
        icon={<ToggleLeft size={16} strokeWidth={1.75} aria-hidden />}
        title={t("keys.pushToTalkTitle")}
        description={t("keys.pushToTalkDescription")}
      />
      <HintCard
        icon={<MousePointerClick size={16} strokeWidth={1.75} aria-hidden />}
        title={t("keys.activeCursorTitle")}
        description={t("keys.activeCursorDescription")}
      />
    </div>
  );
}

function HintCard({
  icon,
  title,
  description,
}: {
  icon: ReactNode;
  title: string;
  description: ReactNode;
}) {
  return (
    <div className="rounded-lg border border-hairline-strong bg-surface-card p-4">
      <div className="mb-2 flex items-center gap-2 text-charcoal">
        {icon}
        <p className="text-caption m-0 font-medium text-ink">{title}</p>
      </div>
      <p className="text-body-sm m-0 text-charcoal">{description}</p>
    </div>
  );
}
