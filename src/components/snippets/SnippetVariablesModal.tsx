import { useEffect, useMemo, useState } from "react";
import { useUiLocale } from "../../i18n/useUiLocale";
import { Button } from "../ui/Button";
import { CopyButton } from "../ui/CopyButton";
import { Modal } from "../ui/Modal";
import { TextInput } from "../ui/TextInput";
import { getSnippetVariables } from "./snippetVariables";

interface SnippetVariablesModalProps {
  open: boolean;
  onClose: () => void;
  busy: boolean;
  userName: string;
  onSaveUserName: (name: string) => Promise<void>;
  onPreview: (content: string) => Promise<string>;
}

function truncateValue(value: string, ellipsis: string, max = 80): string {
  if (value.length <= max) {
    return value;
  }
  return `${value.slice(0, max)}${ellipsis}`;
}

export function SnippetVariablesModal({
  open,
  onClose,
  busy,
  userName,
  onSaveUserName,
  onPreview,
}: SnippetVariablesModalProps) {
  const { t } = useUiLocale();
  const snippetVariables = useMemo(() => getSnippetVariables(t), [t]);
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
        snippetVariables.map(async (variable) => {
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
  }, [open, onPreview, userName, snippetVariables]);

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
      title={t("snippets.variablesModal.title")}
      description={t("snippets.variablesModal.description")}
    >
      <ul className="m-0 flex list-none flex-col gap-3 p-0">
        {snippetVariables.map((variable) => {
          const value = resolved[variable.token];
          const isNom = variable.token === "{{nom}}";
          const displayValue = isNom
            ? userName.trim() || t("common.emDash")
            : value?.trim()
              ? truncateValue(value, t("common.ellipsis"))
              : t("common.emDash");

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
                <CopyButton
                  copied={copiedToken === variable.token}
                  label={t("snippets.variablesModal.copyToken", {
                    token: variable.token,
                  })}
                  copiedLabel={t("common.copied")}
                  disabled={busy}
                  onClick={() => {
                    void copyToken(variable.token);
                  }}
                />
              </div>

              {isNom ? (
                <div className="mt-3">
                  <TextInput
                    label={t("snippets.variablesModal.valueLabel")}
                    value={nomDraft}
                    onChange={(event) => setNomDraft(event.target.value)}
                    onBlur={() => {
                      if (nomDraft.trim() !== userName) {
                        void onSaveUserName(nomDraft);
                      }
                    }}
                    placeholder={t("snippets.variablesModal.name.placeholder")}
                    disabled={busy}
                  />
                </div>
              ) : (
                <p className="text-body-sm m-0 mt-3 text-ink">
                  <span className="text-charcoal">
                    {t("snippets.variablesModal.current")}{" "}
                  </span>
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
          {t("common.close")}
        </Button>
      </div>
    </Modal>
  );
}
