export const SNIPPET_VARIABLES = [
  {
    token: "{{date}}",
    label: "Date",
    hint: "Date du jour (ex. 12 juin 2026)",
  },
  {
    token: "{{clipboard}}",
    label: "Presse-papiers",
    hint: "Contenu actuel du presse-papiers",
  },
  {
    token: "{{nom}}",
    label: "Nom",
    hint: "Votre nom (configurable dans Variables)",
  },
] as const;

const VARIABLE_PATTERN = /\{\{(date|clipboard|nom)\}\}/;

export function containsSnippetVariables(content: string): boolean {
  return VARIABLE_PATTERN.test(content);
}
