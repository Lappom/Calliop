export type SnippetSort = "trigger-asc" | "trigger-desc" | "recent";

export const SNIPPET_SORT_ORDER: SnippetSort[] = [
  "trigger-asc",
  "trigger-desc",
  "recent",
];

export function sortSnippets<T extends { trigger: string; created_at: string }>(
  entries: T[],
  sort: SnippetSort,
): T[] {
  const sorted = [...entries];
  switch (sort) {
    case "trigger-desc":
      sorted.sort((a, b) =>
        b.trigger.localeCompare(a.trigger, "fr", { sensitivity: "base" }),
      );
      break;
    case "recent":
      sorted.sort((a, b) => b.created_at.localeCompare(a.created_at));
      break;
    case "trigger-asc":
    default:
      sorted.sort((a, b) =>
        a.trigger.localeCompare(b.trigger, "fr", { sensitivity: "base" }),
      );
      break;
  }
  return sorted;
}

export function filterSnippets<T extends { trigger: string; content: string }>(
  entries: T[],
  query: string,
): T[] {
  const normalized = query.trim().toLowerCase();
  if (!normalized) {
    return entries;
  }
  return entries.filter(
    (entry) =>
      entry.trigger.toLowerCase().includes(normalized) ||
      entry.content.toLowerCase().includes(normalized),
  );
}
