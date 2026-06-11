import { Check, Copy } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "../ui/Button";
import { Modal } from "../ui/Modal";
import { TextInput } from "../ui/TextInput";
import { SNIPPET_VARIABLES } from "./snippetVariables";

interface SnippetVariablesModalProps {
  open: boolean;
  onClose: () => void;
  busy: boolean;
  userName: string;
  onSaveUserName: (name: string) => Promise<void>;
  onPreview: (content: string) => Promise<string>;
}

function truncateValue(value: string, max = 80): string {
  if (value.length <= max) {
    return value;
  }
  return `${value.slice(0, max)}…`;
}

export function SnippetVariablesModal({
  open,
  onClose,
  busy,
  userName,
  onSaveUserName,
  onPreview,
}: SnippetVariablesModalProps) {
  const [resolved, setResolved] = useState<Record<string, string>>({});
  const [nomDraft, setNomDraft] = useState(userName);
  const [copiedToken, setCopiedToken] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setNomDraft(userName);
    }
  }, [open, userName]);

  useEffect(() => {
    if (!open) {
      return;
    }

    let cancelled = false;

    const load = async () => {
      const entries = await Promise.all(
        SNIPPET_VARIABLES.map(async (variable) => {
          const value = await onPreview(variable.token);
          return [variable.token, value] as const;
        }),
      );
      if (!cancelled) {
        setResolved(Object.fromEntries(entries));
      }
    };

    void load();
    const timer = window.setInterval(() => {
      void load();
    }, 2000);

    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [open, onPreview, userName]);

  const copyToken = async (token: string) => {
    try {
      await navigator.clipboard.writeText(token);
      setCopiedToken(token);
      window.setTimeout(() => setCopiedToken(null), 1500);
    } catch {
      // clipboard unavailable
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      size="md"
      title="Variables"
      description="Valeurs résolues à la dictée. Copiez un jeton pour l'utiliser dans un snippet."
    >
      <ul className="m-0 flex list-none flex-col gap-3 p-0">
        {SNIPPET_VARIABLES.map((variable) => {
          const value = resolved[variable.token];
          const isNom = variable.token === "{{nom}}";
          const displayValue = isNom
            ? userName.trim() || "—"
            : value?.trim()
              ? truncateValue(value)
              : "—";

          return (
            <li
              key={variable.token}
              className="rounded-lg border border-hairline-strong bg-surface-card p-4"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <code className="font-[family-name:var(--font-mono)] text-sm text-accent-blue">
                      {variable.token}
                    </code>
                    <span className="text-caption text-ash">{variable.label}</span>
                  </div>
                  <p className="text-caption m-0 mt-1 text-charcoal">{variable.hint}</p>
                </div>
                <button
                  type="button"
                  aria-label={`Copier ${variable.token}`}
                  disabled={busy}
                  onClick={() => {
                    void copyToken(variable.token);
                  }}
                  className={[
                    "inline-flex size-8 shrink-0 items-center justify-center rounded-md",
                    "border border-transparent text-charcoal transition-colors",
                    "hover:border-hairline-strong hover:bg-surface-elevated hover:text-ink",
                    "disabled:cursor-not-allowed disabled:opacity-40",
                  ].join(" ")}
                >
                  {copiedToken === variable.token ? (
                    <Check size={15} strokeWidth={1.75} className="text-accent-green" />
                  ) : (
                    <Copy size={15} strokeWidth={1.75} />
                  )}
                </button>
              </div>

              {isNom ? (
                <div className="mt-3">
                  <TextInput
                    label="Valeur"
                    value={nomDraft}
                    onChange={(event) => setNomDraft(event.target.value)}
                    onBlur={() => {
                      if (nomDraft.trim() !== userName) {
                        void onSaveUserName(nomDraft);
                      }
                    }}
                    placeholder="Ex. Marie Dupont"
                    disabled={busy}
                  />
                </div>
              ) : (
                <p className="text-body-sm m-0 mt-3 text-ink">
                  <span className="text-charcoal">Actuel : </span>
                  <span className="font-[family-name:var(--font-mono)] text-body">
                    {displayValue}
                  </span>
                </p>
              )}
            </li>
          );
        })}
      </ul>

      <div className="mt-6 flex justify-end">
        <Button type="button" variant="primary" disabled={busy} onClick={onClose}>
          Fermer
        </Button>
      </div>
    </Modal>
  );
}
