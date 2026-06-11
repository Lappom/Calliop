/** Duration values in seconds (Motion API). */
export const MOTION_DURATION = {
  instant: 0,
  fast: 0.15,
  base: 0.25,
  slow: 0.4,
} as const;

/** Cubic-bezier tuples for editorial, restrained motion. */
export const MOTION_EASE = {
  editorial: [0.16, 1, 0.3, 1] as const,
  enter: [0.22, 1, 0.36, 1] as const,
  springSoft: [0.34, 1.2, 0.64, 1] as const,
} as const;

export const MOTION_STAGGER = {
  /** Default section stagger — 40ms per item, total under 300ms for ~6 items. */
  children: 0.04,
  /** Editorial page reveals — slightly slower hierarchy. */
  editorial: 0.05,
} as const;

/** Shared layout transition for paired sidebar indicators (bg + accent bar). */
export const LAYOUT_TRANSITION = {
  type: "spring" as const,
  stiffness: 480,
  damping: 38,
  mass: 0.85,
};

export const LAYOUT_TRANSITION_REDUCED = {
  duration: 0,
} as const;
