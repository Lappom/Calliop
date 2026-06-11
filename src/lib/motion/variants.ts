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
