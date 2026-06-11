import { motion } from "motion/react";
import type { ReactNode } from "react";
import { listRowVariants, pickVariants } from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";

interface AnimatedListItemProps {
  index: number;
  children: ReactNode;
  className?: string;
}

export function AnimatedListItem({
  index,
  children,
  className,
}: AnimatedListItemProps) {
  const reducedMotion = useReducedMotion();
  const variants = pickVariants(listRowVariants, reducedMotion);

  return (
    <motion.li
      custom={index}
      variants={variants}
      initial="initial"
      animate="animate"
      className={className}
    >
      {children}
    </motion.li>
  );
}
