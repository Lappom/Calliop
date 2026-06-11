import type { Variants } from "motion/react";
import { MOTION_DURATION, MOTION_EASE, MOTION_STAGGER } from "./presets";

export const pageVariants: Variants = {
  initial: { opacity: 0, y: 8 },
  animate: {
    opacity: 1,
    y: 0,
    transition: {
      duration: MOTION_DURATION.base,
      ease: MOTION_EASE.editorial,
    },
  },
  exit: {
    opacity: 0,
    transition: {
      duration: MOTION_DURATION.fast,
      ease: "linear",
    },
  },
};

export const reducedMotionVariants: Variants = {
  initial: { opacity: 1 },
  animate: { opacity: 1 },
  exit: { opacity: 1 },
};

export const staggerContainerVariants: Variants = {
  initial: {},
  animate: {
    transition: {
      staggerChildren: MOTION_STAGGER.children,
    },
  },
};

export function createStaggerContainerVariants(
  staggerChildren: number,
): Variants {
  return {
    initial: {},
    animate: {
      transition: { staggerChildren },
    },
  };
}

export const fadeUpVariants: Variants = {
  initial: { opacity: 0, y: 8 },
  animate: {
    opacity: 1,
    y: 0,
    transition: {
      duration: MOTION_DURATION.base,
      ease: MOTION_EASE.editorial,
    },
  },
};

/** Opacity-only — use inside PageTransition to avoid stacked vertical motion. */
export const staggerFadeVariants: Variants = {
  initial: { opacity: 0 },
  animate: {
    opacity: 1,
    transition: {
      duration: MOTION_DURATION.base,
      ease: MOTION_EASE.enter,
    },
  },
};

const listRowExit = {
  opacity: 0,
  transition: {
    duration: MOTION_DURATION.fast,
    ease: MOTION_EASE.enter,
  },
} as const;

/** Table/list rows — capped stagger delay via `custom` index; fast opacity exit on remove. */
export const listRowVariants: Variants = {
  initial: { opacity: 0 },
  animate: (index: number) => ({
    opacity: 1,
    transition: {
      duration: MOTION_DURATION.fast,
      ease: MOTION_EASE.enter,
      delay: Math.min(index * MOTION_STAGGER.children, 0.24),
    },
  }),
  exit: listRowExit,
};

/** Empty state panels — subtle scale + opacity (rare surfaces). */
export const emptyStateVariants: Variants = {
  initial: { opacity: 0, scale: 0.98 },
  animate: {
    opacity: 1,
    scale: 1,
    transition: {
      duration: MOTION_DURATION.base,
      ease: MOTION_EASE.editorial,
    },
  },
};

const overlayTransition = {
  duration: MOTION_DURATION.base,
  ease: MOTION_EASE.enter,
} as const;

const overlayExitTransition = {
  duration: MOTION_DURATION.fast,
  ease: MOTION_EASE.enter,
} as const;

export const modalBackdropVariants: Variants = {
  initial: { opacity: 0 },
  animate: { opacity: 1, transition: overlayTransition },
  exit: { opacity: 0, transition: overlayExitTransition },
};

export const modalPanelVariants: Variants = {
  initial: { opacity: 0, scale: 0.96 },
  animate: { opacity: 1, scale: 1, transition: overlayTransition },
  exit: { opacity: 0, scale: 0.98, transition: overlayExitTransition },
};

export const dropdownPanelVariants: Variants = {
  initial: { opacity: 0, y: -4 },
  animate: { opacity: 1, y: 0, transition: overlayTransition },
  exit: { opacity: 0, y: -2, transition: overlayExitTransition },
};

export const toastVariants: Variants = {
  initial: { opacity: 0, y: 12 },
  animate: { opacity: 1, y: 0, transition: overlayTransition },
  exit: { opacity: 0, y: 8, transition: overlayExitTransition },
};

export function pickVariants(
  variants: Variants,
  reducedMotion: boolean,
): Variants {
  return reducedMotion ? reducedMotionVariants : variants;
}

const stepSlideTransition = {
  duration: MOTION_DURATION.base,
  ease: MOTION_EASE.enter,
} as const;

const stepSlideExitTransition = {
  duration: MOTION_DURATION.fast,
  ease: MOTION_EASE.enter,
} as const;

/** Direction: 1 = forward, -1 = back */
export const onboardingStepVariants: Variants = {
  initial: (direction: number) => ({
    opacity: 0,
    x: direction > 0 ? 20 : -20,
  }),
  animate: {
    opacity: 1,
    x: 0,
    transition: stepSlideTransition,
  },
  exit: (direction: number) => ({
    opacity: 0,
    x: direction > 0 ? -12 : 12,
    transition: stepSlideExitTransition,
  }),
};
