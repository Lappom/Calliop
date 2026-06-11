/** Chart palette aligned with DESIGN.md semantic accents */
export const CHART_COLORS = {
  blue: "#3b9eff",
  green: "#11ff99",
  orange: "#ff801f",
  yellow: "#ffc53d",
  red: "#ff2047",
  hairline: "rgba(255, 255, 255, 0.14)",
  grid: "rgba(255, 255, 255, 0.06)",
  label: "rgba(252, 253, 255, 0.7)",
  muted: "#888e90",
} as const;

export const APP_SEGMENT_COLORS = [
  CHART_COLORS.blue,
  CHART_COLORS.green,
  CHART_COLORS.orange,
  CHART_COLORS.yellow,
  CHART_COLORS.red,
] as const;

export function formatShortDate(isoDate: string, intlLocale: string): string {
  const date = new Date(`${isoDate}T12:00:00`);
  if (Number.isNaN(date.getTime())) {
    return isoDate;
  }
  return new Intl.DateTimeFormat(intlLocale, {
    weekday: "short",
    day: "numeric",
  }).format(date);
}

export function formatShortTime(iso: string, intlLocale: string): string {
  const normalized = iso.includes("T") ? iso : `${iso.replace(" ", "T")}Z`;
  const date = new Date(normalized);
  if (Number.isNaN(date.getTime())) {
    return "—";
  }
  return new Intl.DateTimeFormat(intlLocale, {
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

/** Latency chart — include seconds so nearby dictations stay distinct on the axis. */
export function formatLatencyAxisTime(iso: string, intlLocale: string): string {
  const normalized = iso.includes("T") ? iso : `${iso.replace(" ", "T")}Z`;
  const date = new Date(normalized);
  if (Number.isNaN(date.getTime())) {
    return "—";
  }
  return new Intl.DateTimeFormat(intlLocale, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(date);
}
