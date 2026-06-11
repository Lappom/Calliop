import { BadgePill } from "../ui/BadgePill";
import { Card } from "../ui/Card";
import { SectionGlow } from "../layout/SectionGlow";

const PROFILE_EXAMPLES = [
  { app: "Slack", tone: "Casual", description: "Messages courts, ton détendu" },
  { app: "Outlook", tone: "Formel", description: "Formules de politesse, structure mail" },
  { app: "VS Code", tone: "Technique", description: "Commits, commentaires concis" },
];

export function ContexteView() {
  return (
    <div className="flex flex-col gap-8">
      <header>
        <div className="mb-2 flex items-center gap-3">
          <h1 className="text-heading-md text-ink">Contexte</h1>
          <BadgePill>Bientôt</BadgePill>
        </div>
        <p className="text-body-sm text-charcoal">
          Adaptez le ton de la dictée selon l&apos;application active.
        </p>
      </header>

      <SectionGlow glow="green">
        <div className="grid gap-4 sm:grid-cols-3">
          {PROFILE_EXAMPLES.map((profile) => (
            <Card key={profile.app} variant="bordered" className="p-5">
              <p className="text-heading-sm mb-1 text-ink">{profile.app}</p>
              <BadgePill className="mb-3">{profile.tone}</BadgePill>
              <p className="text-caption text-ash">{profile.description}</p>
            </Card>
          ))}
        </div>
        <p className="text-caption mt-4 text-ash">
          Phase 3d — détection fenêtre active, profils configurables par
          application.
        </p>
      </SectionGlow>
    </div>
  );
}
