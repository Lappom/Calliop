import { AnimatePresence, motion } from "motion/react";
import type { ReactNode } from "react";
import { listRowVariants, pickVariants } from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";

export const TABLE_ROW_CLASS =
  "group border-b border-divider-soft transition-colors last:border-b-0 hover:bg-surface-elevated/50";

interface AnimatedTableBodyProps<T> {
  items: T[];
  getRowKey: (item: T) => string | number;
  renderRow: (item: T) => ReactNode;
  rowClassName?: string;
}

export function AnimatedTableBody<T>({
  items,
  getRowKey,
  renderRow,
  rowClassName = TABLE_ROW_CLASS,
}: AnimatedTableBodyProps<T>) {
  const reducedMotion = useReducedMotion();
  const variants = pickVariants(listRowVariants, reducedMotion);

  return (
    <tbody>
      <AnimatePresence initial={false}>
        {items.map((item, index) => (
          <motion.tr
            key={getRowKey(item)}
            custom={index}
            variants={variants}
            initial="initial"
            animate="animate"
            exit="exit"
            className={rowClassName}
          >
            {renderRow(item)}
          </motion.tr>
        ))}
      </AnimatePresence>
    </tbody>
  );
}
