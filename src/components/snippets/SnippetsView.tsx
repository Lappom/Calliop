import { BadgePill } from "../ui/BadgePill";
import { Card } from "../ui/Card";
import { SectionGlow } from "../layout/SectionGlow";

export function SnippetsView() {
  return (
    <div className="flex flex-col gap-8">
      <header>
        <div className="mb-2 flex items-center gap-3">
          <h1 className="text-heading-md text-ink">Snippets</h1>
          <BadgePill>Bientôt</BadgePill>
        </div>
        <p className="text-body-sm text-charcoal">
          Définissez des déclencheurs vocaux qui insèrent un texte complet.
        </p>
      </header>

      <SectionGlow glow="orange" className="mb-2">
        <Card variant="bordered" className="space-y-4 p-6">
          <p className="text-body-md text-ink">Exemple de snippet</p>
          <div className="rounded-md border border-hairline bg-surface-deep px-4 py-3">
            <p className="text-code-md m-0 text-charcoal">
              <span className="text-accent-orange">&quot;mon calendrier&quot;</span>
              {" → "}
              <span className="text-ink">
                Voici mon lien Calendly : calendly.com/…
              </span>
            </p>
          </div>
          <p className="text-caption text-ash">
            Phase 3c — matching dans le post-traitement LLM, import / export
            JSON.
          </p>
        </Card>
      </SectionGlow>
    </div>
  );
}
