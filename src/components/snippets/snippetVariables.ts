import type { TFunction } from "i18next";

export const SNIPPET_VARIABLE_TOKENS = ["{{date}}", "{{clipboard}}", "{{nom}}"] as const;

const VARIABLE_PATTERN = /\{\{(date|clipboard|nom)\}\}/;

export function containsSnippetVariables(content: string): boolean {
  return VARIABLE_PATTERN.test(content);
}

export function getSnippetVariables(t: TFunction) {
  return [
    {
      token: "{{date}}",
      label: t("snippets.variablesModal.date.label"),
      hint: t("snippets.variablesModal.date.hint"),
    },
    {
      token: "{{clipboard}}",
      label: t("snippets.variablesModal.clipboard.label"),
      hint: t("snippets.variablesModal.clipboard.hint"),
    },
    {
      token: "{{nom}}",
      label: t("snippets.variablesModal.name.label"),
      hint: t("snippets.variablesModal.name.hint"),
    },
  ] as const;
}
