import { AnimatePresence, motion } from "motion/react";
import { createContext, useContext, useEffect, type ReactNode } from "react";
import { pageVariants, reducedMotionVariants } from "../../lib/motion/variants";
import { markViewVisited } from "../../lib/motion/useViewReveal";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";

const ViewRevealContext = createContext("");

export function useViewRevealKey(): string {
  return useContext(ViewRevealContext);
}

interface PageTransitionProps {
  viewKey: string;
  children: ReactNode;
}

export function PageTransition({ viewKey, children }: PageTransitionProps) {
  const reducedMotion = useReducedMotion();
  const variants = reducedMotion ? reducedMotionVariants : pageVariants;

  useEffect(() => {
    return () => {
      markViewVisited(viewKey);
    };
  }, [viewKey]);

  return (
    <ViewRevealContext.Provider value={viewKey}>
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
    </ViewRevealContext.Provider>
  );
}
