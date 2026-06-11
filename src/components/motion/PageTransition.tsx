import { AnimatePresence, motion } from "motion/react";
import type { ReactNode } from "react";
import { pageVariants, reducedMotionVariants } from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";

interface PageTransitionProps {
  viewKey: string;
  children: ReactNode;
}

export function PageTransition({ viewKey, children }: PageTransitionProps) {
  const reducedMotion = useReducedMotion();
  const variants = reducedMotion ? reducedMotionVariants : pageVariants;

  return (
    <AnimatePresence mode="wait" initial={false}>
      <motion.div
        key={viewKey}
        variants={variants}
        initial="initial"
        animate="animate"
        exit="exit"
        className="flex flex-col"
      >
        {children}
      </motion.div>
    </AnimatePresence>
  );
}
