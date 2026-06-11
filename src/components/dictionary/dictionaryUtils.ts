import type { TFunction } from "i18next";
import type {
  DictionarySource,
  DictionaryWord,
} from "../../hooks/useDictionary";

export type DictionarySort = "alpha-asc" | "alpha-desc" | "recent" | "source";

export type DictionarySourceFilter = "all" | DictionarySource;

export function getDictionarySortLabels(
  t: TFunction,
): Record<DictionarySort, string> {
  return {
    "alpha-asc": t("dictionary.sort.alphaAsc"),
    "alpha-desc": t("dictionary.sort.alphaDesc"),
    recent: t("dictionary.sort.recent"),
    source: t("dictionary.sort.source"),
  };
}

export function getDictionaryFilterLabels(
  t: TFunction,
): Record<DictionarySourceFilter, string> {
  return {
    all: t("dictionary.filter.all"),
    manual: t("dictionary.filter.manual"),
    learned: t("dictionary.filter.learned"),
  };
}

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

export function getSourceMeta(
  t: TFunction,
): Record<
  DictionarySource,
  { label: string; accent: string; description: string }
> {
  return {
    manual: {
      label: t("dictionary.source.manual.label"),
      accent: "var(--color-accent-blue)",
      description: t("dictionary.source.manual.description"),
    },
    learned: {
      label: t("dictionary.source.learned.label"),
      accent: "var(--color-accent-green)",
      description: t("dictionary.source.learned.description"),
    },
  };
}

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
  intlLocale: string,
): DictionaryWord[] {
  const normalized = query.trim().toLocaleLowerCase(intlLocale);
  return words.filter((entry) => {
    if (sourceFilter !== "all" && entry.source !== sourceFilter) {
      return false;
    }
    if (!normalized) {
      return true;
    }
    const wordMatch = entry.word
      .toLocaleLowerCase(intlLocale)
      .includes(normalized);
    const misspellingMatch = entry.misspelling
      ?.toLocaleLowerCase(intlLocale)
      .includes(normalized);
    return wordMatch || Boolean(misspellingMatch);
  });
}

export function sortDictionaryWords(
  words: DictionaryWord[],
  sort: DictionarySort,
  intlLocale: string,
): DictionaryWord[] {
  const sorted = [...words];
  switch (sort) {
    case "alpha-desc":
      sorted.sort((a, b) =>
        b.word.localeCompare(a.word, intlLocale, { sensitivity: "base" }),
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
        return a.word.localeCompare(b.word, intlLocale, { sensitivity: "base" });
      });
      break;
    case "alpha-asc":
    default:
      sorted.sort((a, b) =>
        a.word.localeCompare(b.word, intlLocale, { sensitivity: "base" }),
      );
      break;
  }
  return sorted;
}

export function formatDictionaryDate(iso: string, intlLocale: string): string {
  const date = parseDictionaryDate(iso);
  if (!date) {
    return iso;
  }
  return new Intl.DateTimeFormat(intlLocale, {
    day: "numeric",
    month: "short",
    year: "numeric",
  }).format(date);
}

function parseDictionaryDate(iso: string): Date | null {
  const date = new Date(iso.includes("T") ? iso : `${iso.replace(" ", "T")}Z`);
  return Number.isNaN(date.getTime()) ? null : date;
}
