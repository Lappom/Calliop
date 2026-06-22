import { motion } from "motion/react";
import {
  Children,
  Fragment,
  isValidElement,
  useId,
  type ReactNode,
} from "react";
import { MOTION_STAGGER } from "../../lib/motion/presets";
import { useViewReveal } from "../../lib/motion/useViewReveal";
import {
  createStaggerContainerVariants,
  fadeUpVariants,
  reducedMotionVariants,
  staggerContainerVariants,
  staggerFadeVariants,
} from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";
import { useViewRevealKey } from "./PageTransition";

type StaggerItemMotion = "fadeUp" | "fade";

interface StaggerProps {
  children: ReactNode;
  className?: string;
  /** fade = opacity only (nested in PageTransition); fadeUp = opacity + y */
  itemMotion?: StaggerItemMotion;
  staggerDelay?: number;
  itemClassName?: string;
  /** Override view key for first-visit reveal; defaults to PageTransition context */
  viewKey?: string;
}

const itemVariantsByMotion: Record<StaggerItemMotion, typeof fadeUpVariants> = {
  fadeUp: fadeUpVariants,
  fade: staggerFadeVariants,
};

/** React.Children.toArray does not flatten fragments — unwrap them for flex gap. */
function flattenStaggerChildren(children: ReactNode): ReactNode[] {
  const items: ReactNode[] = [];

  Children.forEach(children, (child) => {
    if (child == null || typeof child === "boolean") return;

    if (isValidElement(child) && child.type === Fragment) {
      items.push(
        ...flattenStaggerChildren(
          (child.props as { children?: ReactNode }).children,
        ),
      );
      return;
    }

    items.push(child);
  });

  return items;
}

export function Stagger({
  children,
  className = "",
  itemMotion = "fadeUp",
  staggerDelay = MOTION_STAGGER.children,
  itemClassName = "",
  viewKey: viewKeyProp,
}: StaggerProps) {
  const contextViewKey = useViewRevealKey();
  const viewKey = viewKeyProp ?? contextViewKey;
  const staggerInstanceId = useId();
  const revealFromHook = useViewReveal(
    viewKey || "__stagger__",
    viewKey ? staggerInstanceId : undefined,
  );
  const revealOnFirstVisit = viewKey ? revealFromHook : true;
  const reducedMotion = useReducedMotion();
  const shouldAnimate = !reducedMotion && revealOnFirstVisit;

  const containerVariants = shouldAnimate
    ? staggerDelay === MOTION_STAGGER.children
      ? staggerContainerVariants
      : createStaggerContainerVariants(staggerDelay)
    : reducedMotionVariants;
  const itemVariants = shouldAnimate
    ? itemVariantsByMotion[itemMotion]
    : reducedMotionVariants;

  const items = flattenStaggerChildren(children);

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
          className={itemClassName}
        >
          {child}
        </motion.div>
      ))}
    </motion.div>
  );
}
