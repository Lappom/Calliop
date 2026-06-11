export const CHART_VIEW_WIDTH = 400;
export const CHART_VIEW_HEIGHT = 200;
export const CHART_PADDING = {
  top: 16,
  right: 12,
  bottom: 36,
  left: 40,
} as const;

export interface BarLayout {
  barWidth: number;
  gap: number;
  groupOffset: number;
}

export function computeBarLayout(
  barCount: number,
  plotWidth: number,
  options: { maxBarWidth?: number; gap?: number } = {},
): BarLayout {
  const gap = options.gap ?? 8;
  const maxBarWidth = options.maxBarWidth ?? 28;

  if (barCount <= 0) {
    return { barWidth: maxBarWidth, gap, groupOffset: 0 };
  }

  const naturalWidth = (plotWidth - gap * (barCount - 1)) / barCount;
  const barWidth = Math.min(maxBarWidth, naturalWidth);
  const groupWidth = barCount * barWidth + gap * (barCount - 1);
  const groupOffset = Math.max(0, (plotWidth - groupWidth) / 2);

  return { barWidth, gap, groupOffset };
}

export function plotDimensions() {
  const plotWidth =
    CHART_VIEW_WIDTH - CHART_PADDING.left - CHART_PADDING.right;
  const plotHeight =
    CHART_VIEW_HEIGHT - CHART_PADDING.top - CHART_PADDING.bottom;
  return { plotWidth, plotHeight };
}

export function gridLineRatios(count: number): number[] {
  if (count <= 1) {
    return [0, 1];
  }
  return Array.from({ length: count }, (_, index) => index / (count - 1));
}
