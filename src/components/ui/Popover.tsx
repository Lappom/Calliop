import { AnimatePresence, motion } from "motion/react";
import {
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { dropdownPanelVariants, pickVariants } from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";

type PopoverAlign = "start" | "end";

interface PopoverProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  trigger: ReactNode;
  children: ReactNode;
  align?: PopoverAlign;
  /** Gap between trigger and panel in px */
  offset?: number;
  menuLabel?: string;
}

export function Popover({
  open,
  onOpenChange,
  trigger,
  children,
  align = "end",
  offset = 8,
  menuLabel,
}: PopoverProps) {
  const menuId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const reducedMotion = useReducedMotion();
  const panelVariants = pickVariants(dropdownPanelVariants, reducedMotion);
  const [panelStyle, setPanelStyle] = useState({ top: 0, left: 0, minWidth: 0 });

  const close = useCallback(() => {
    onOpenChange(false);
  }, [onOpenChange]);

  const updatePanelPosition = useCallback(() => {
    const root = rootRef.current;
    if (!root) return;

    const triggerEl = root.querySelector<HTMLElement>("[data-popover-trigger]");
    if (!triggerEl) return;

    const rect = triggerEl.getBoundingClientRect();
    setPanelStyle({
      top: rect.bottom + offset,
      left: align === "end" ? rect.right : rect.left,
      minWidth: rect.width,
    });
  }, [align, offset]);

  useEffect(() => {
    if (!open) return;

    updatePanelPosition();
    panelRef.current?.focus();

    const handleScrollOrResize = () => updatePanelPosition();
    window.addEventListener("resize", handleScrollOrResize);
    window.addEventListener("scroll", handleScrollOrResize, true);

    return () => {
      window.removeEventListener("resize", handleScrollOrResize);
      window.removeEventListener("scroll", handleScrollOrResize, true);
    };
  }, [open, updatePanelPosition]);

  useEffect(() => {
    if (!open) return;

    const handlePointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (rootRef.current?.contains(target) || panelRef.current?.contains(target)) {
        return;
      }
      close();
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        close();
        rootRef.current
          ?.querySelector<HTMLElement>("[data-popover-trigger]")
          ?.focus();
      }
    };

    document.addEventListener("mousedown", handlePointerDown);
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [close, open]);

  const handlePanelKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      close();
      rootRef.current
        ?.querySelector<HTMLElement>("[data-popover-trigger]")
        ?.focus();
    }
  };

  const transformOrigin = align === "end" ? "top right" : "top left";

  return (
    <div ref={rootRef} className="relative inline-flex">
      {trigger}
      {createPortal(
        <AnimatePresence>
          {open && (
            <div
              style={{
                position: "fixed",
                top: panelStyle.top,
                left: panelStyle.left,
                minWidth: panelStyle.minWidth,
                zIndex: 50,
                transform: align === "end" ? "translateX(-100%)" : undefined,
              }}
            >
              <motion.div
                ref={panelRef}
                id={menuId}
                role={menuLabel ? "menu" : undefined}
                aria-label={menuLabel}
                tabIndex={-1}
                variants={panelVariants}
                initial="initial"
                animate="animate"
                exit="exit"
                onKeyDown={handlePanelKeyDown}
                style={{ transformOrigin }}
                className="outline-none"
              >
                {children}
              </motion.div>
            </div>
          )}
        </AnimatePresence>,
        document.body,
      )}
    </div>
  );
}
