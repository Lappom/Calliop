import { motion } from "motion/react";
import type { ReactNode } from "react";
import { pickVariants, staggerFadeVariants } from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";

interface NoResultsCardProps {
  children: ReactNode;
  className?: string;
}

export function NoResultsCard({
  children,
  className = "",
}: NoResultsCardProps) {
  const reducedMotion = useReducedMotion();
  const variants = pickVariants(staggerFadeVariants, reducedMotion);

  return (
    <motion.div
      variants={variants}
      initial="initial"
      animate="animate"
      className={[
        "rounded-lg border border-hairline-strong bg-surface-card px-4 py-8 text-center",
        className,
      ].join(" ")}
    >
      {children}
    </motion.div>
  );
}
