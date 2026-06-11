import type {
  DictionarySource,
  DictionaryWord,
} from "../../hooks/useDictionary";

export type DictionarySort = "alpha-asc" | "alpha-desc" | "recent" | "source";

export type DictionarySourceFilter = "all" | DictionarySource;

export const DICTIONARY_SORT_LABELS: Record<DictionarySort, string> = {
  "alpha-asc": "Tri A → Z",
  "alpha-desc": "Tri Z → A",
  recent: "Plus récents",
  source: "Par source",
};

export const SOURCE_FILTER_LABELS: Record<DictionarySourceFilter, string> = {
  all: "Tous",
  manual: "Manuels",
  learned: "Appris",
};

export const DICTIONARY_SORT_ORDER: DictionarySort[] = [
  "alpha-asc",
  "alpha-desc",
  "recent",
  "source",
];

export const SOURCE_FILTER_ORDER: DictionarySourceFilter[] = [
  "all",
  "manual",
  "learned",
];

export const SOURCE_META: Record<
  DictionarySource,
  { label: string; accent: string; description: string }
> = {
  manual: {
    label: "Manuel",
    accent: "var(--color-accent-blue)",
    description: "Ajouté depuis cette page",
  },
  learned: {
    label: "Appris",
    accent: "var(--color-accent-green)",
    description: "Détecté via une correction de dictée",
  },
};

export function nextDictionarySort(current: DictionarySort): DictionarySort {
  if (current === "alpha-asc") return "alpha-desc";
  if (current === "alpha-desc") return "recent";
  if (current === "recent") return "source";
  return "alpha-asc";
}

export function filterDictionaryWords(
  words: DictionaryWord[],
  query: string,
  sourceFilter: DictionarySourceFilter,
): DictionaryWord[] {
  const normalized = query.trim().toLocaleLowerCase("fr-FR");
  return words.filter((entry) => {
    if (sourceFilter !== "all" && entry.source !== sourceFilter) {
      return false;
    }
    if (!normalized) {
      return true;
    }
    const wordMatch = entry.word
      .toLocaleLowerCase("fr-FR")
      .includes(normalized);
    const misspellingMatch = entry.misspelling
      ?.toLocaleLowerCase("fr-FR")
      .includes(normalized);
    return wordMatch || Boolean(misspellingMatch);
  });
}

export function sortDictionaryWords(
  words: DictionaryWord[],
  sort: DictionarySort,
): DictionaryWord[] {
  const sorted = [...words];
  switch (sort) {
    case "alpha-desc":
      sorted.sort((a, b) =>
        b.word.localeCompare(a.word, "fr-FR", { sensitivity: "base" }),
      );
      break;
    case "recent":
      sorted.sort((a, b) => b.created_at.localeCompare(a.created_at));
      break;
    case "source":
      sorted.sort((a, b) => {
        const sourceOrder =
          a.source === b.source ? 0 : a.source === "manual" ? -1 : 1;
        if (sourceOrder !== 0) {
          return sourceOrder;
        }
        return a.word.localeCompare(b.word, "fr-FR", { sensitivity: "base" });
      });
      break;
    case "alpha-asc":
    default:
      sorted.sort((a, b) =>
        a.word.localeCompare(b.word, "fr-FR", { sensitivity: "base" }),
      );
      break;
  }
  return sorted;
}

export function formatDictionaryDate(iso: string): string {
  const date = parseDictionaryDate(iso);
  if (!date) {
    return iso;
  }
  return new Intl.DateTimeFormat("fr-FR", {
    day: "numeric",
    month: "short",
    year: "numeric",
  }).format(date);
}

function parseDictionaryDate(iso: string): Date | null {
  const date = new Date(iso.includes("T") ? iso : `${iso.replace(" ", "T")}Z`);
  return Number.isNaN(date.getTime()) ? null : date;
}
