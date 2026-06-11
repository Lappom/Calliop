import type { ReactNode } from "react";
import { Mic, MousePointerClick, ToggleLeft } from "lucide-react";
import { Kbd } from "../ui/Kbd";

export function MainHotkeyGuide() {
  return (
    <div className="grid gap-3 sm:grid-cols-3">
      <HintCard
        icon={<Mic size={16} strokeWidth={1.75} aria-hidden />}
        title="Toggle"
        description={
          <>
            Appuyez sur <Kbd>Alt</Kbd> + <Kbd>Espace</Kbd> pour démarrer, puis
            réappuyez pour arrêter.
          </>
        }
      />
      <HintCard
        icon={<ToggleLeft size={16} strokeWidth={1.75} aria-hidden />}
        title="Push-to-talk"
        description="Maintenez le raccourci pendant que vous parlez, relâchez pour transcrire."
      />
      <HintCard
        icon={<MousePointerClick size={16} strokeWidth={1.75} aria-hidden />}
        title="Curseur actif"
        description="Le texte est inséré là où se trouve le curseur — Notepad, Word, navigateur…"
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
