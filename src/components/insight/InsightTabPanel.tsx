import { AnimatePresence, motion } from "motion/react";
import type { ReactNode } from "react";
import { MOTION_DURATION, MOTION_EASE } from "../../lib/motion/presets";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";
import { insightTabPanelId, type InsightTabId } from "./insightTabs";

interface InsightTabPanelProps {
  activeTab: InsightTabId;
  children: ReactNode;
}

const panelVariants = {
  initial: { opacity: 0, y: 6 },
  animate: {
    opacity: 1,
    y: 0,
    transition: {
      duration: MOTION_DURATION.base,
      ease: MOTION_EASE.enter,
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

const panelVariantsReduced = {
  initial: { opacity: 1 },
  animate: { opacity: 1 },
  exit: { opacity: 1 },
};

export function InsightTabPanel({ activeTab, children }: InsightTabPanelProps) {
  const reducedMotion = useReducedMotion();
  const variants = reducedMotion ? panelVariantsReduced : panelVariants;

  return (
    <AnimatePresence mode="wait" initial={false}>
      <motion.div
        key={activeTab}
        id={insightTabPanelId(activeTab)}
        role="tabpanel"
        aria-labelledby={`insight-tab-${activeTab}`}
        variants={variants}
        initial="initial"
        animate="animate"
        exit="exit"
        className="flex flex-col gap-8"
      >
        {children}
      </motion.div>
    </AnimatePresence>
  );
}
