import type { CSSProperties } from "react";
import { CHART_COLORS } from "./chartTheme";

export const RECHARTS_AXIS_TICK = {
  fill: CHART_COLORS.muted,
  fontSize: 10,
  fontFamily: "var(--font-mono)",
} as const;

export const RECHARTS_AXIS_TICK_UI = {
  fill: CHART_COLORS.muted,
  fontSize: 10,
  fontFamily: "var(--font-ui)",
} as const;

export const RECHARTS_MARGIN = {
  top: 8,
  right: 12,
  left: 0,
  bottom: 4,
} as const;

export const rechartsTooltipStyle: CSSProperties = {
  backgroundColor: "var(--color-surface-elevated)",
  border: "1px solid var(--color-hairline-strong)",
  borderRadius: "var(--rounded-md)",
  padding: "8px 12px",
  fontSize: "12px",
  fontFamily: "var(--font-ui)",
  color: "var(--color-charcoal)",
};
