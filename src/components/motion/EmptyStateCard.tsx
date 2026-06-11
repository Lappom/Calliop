import { motion } from "motion/react";
import type { ReactNode } from "react";
import type { GlowColor } from "../layout/glowSurface";
import { SectionGlow } from "../layout/SectionGlow";
import { emptyStateVariants, pickVariants } from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";

interface EmptyStateCardProps {
  glow: GlowColor;
  children: ReactNode;
  className?: string;
}

export function EmptyStateCard({
  glow,
  children,
  className = "",
}: EmptyStateCardProps) {
  const reducedMotion = useReducedMotion();
  const variants = pickVariants(emptyStateVariants, reducedMotion);

  return (
    <SectionGlow glow={glow}>
      <motion.div
        variants={variants}
        initial="initial"
        animate="animate"
        className={[
          "rounded-lg border border-hairline-strong bg-surface-card p-6 sm:p-8",
          className,
        ].join(" ")}
      >
        {children}
      </motion.div>
    </SectionGlow>
  );
}
