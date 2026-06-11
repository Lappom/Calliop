import { AnimatePresence, motion } from "motion/react";
import type { ReactNode } from "react";
import { onboardingStepVariants, pickVariants } from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";

interface OnboardingStepTransitionProps {
  stepKey: string | number;
  direction: number;
  children: ReactNode;
}

export function OnboardingStepTransition({
  stepKey,
  direction,
  children,
}: OnboardingStepTransitionProps) {
  const reducedMotion = useReducedMotion();
  const variants = pickVariants(onboardingStepVariants, reducedMotion);

  return (
    <AnimatePresence mode="wait" custom={direction} initial={false}>
      <motion.div
        key={stepKey}
        custom={direction}
        variants={variants}
        initial="initial"
        animate="animate"
        exit="exit"
      >
        {children}
      </motion.div>
    </AnimatePresence>
  );
}
