import { motion } from "motion/react";
import { Children, isValidElement, type ReactNode } from "react";
import { MOTION_STAGGER } from "../../lib/motion/presets";
import {
  createStaggerContainerVariants,
  fadeUpVariants,
  reducedMotionVariants,
  staggerContainerVariants,
  staggerFadeVariants,
} from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";

type StaggerItemMotion = "fadeUp" | "fade";

interface StaggerProps {
  children: ReactNode;
  className?: string;
  /** fade = opacity only (nested in PageTransition); fadeUp = opacity + y */
  itemMotion?: StaggerItemMotion;
  staggerDelay?: number;
}

const itemVariantsByMotion: Record<StaggerItemMotion, typeof fadeUpVariants> = {
  fadeUp: fadeUpVariants,
  fade: staggerFadeVariants,
};

export function Stagger({
  children,
  className = "",
  itemMotion = "fadeUp",
  staggerDelay = MOTION_STAGGER.children,
}: StaggerProps) {
  const reducedMotion = useReducedMotion();
  const containerVariants = reducedMotion
    ? reducedMotionVariants
    : staggerDelay === MOTION_STAGGER.children
      ? staggerContainerVariants
      : createStaggerContainerVariants(staggerDelay);
  const itemVariants = reducedMotion
    ? reducedMotionVariants
    : itemVariantsByMotion[itemMotion];

  const items = Children.toArray(children);

  return (
    <motion.div
      variants={containerVariants}
      initial="initial"
      animate="animate"
      className={className}
    >
      {items.map((child, index) => (
        <motion.div
          key={isValidElement(child) && child.key != null ? child.key : index}
          variants={itemVariants}
        >
          {child}
        </motion.div>
      ))}
    </motion.div>
  );
}
