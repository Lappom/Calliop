import { Search } from "lucide-react";
import { motion } from "motion/react";
import {
  useCallback,
  useEffect,
  useId,
  useRef,
  type ChangeEvent,
} from "react";
import { MOTION_DURATION, MOTION_EASE } from "../../lib/motion/presets";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";

const EXPANDED_WIDTH_PX = 240;

interface ExpandableSearchFieldProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  value: string;
  onChange: (value: string) => void;
  label: string;
  placeholder: string;
  disabled?: boolean;
}

export function ExpandableSearchField({
  open,
  onOpenChange,
  value,
  onChange,
  label,
  placeholder,
  disabled = false,
}: ExpandableSearchFieldProps) {
  const inputId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const reducedMotion = useReducedMotion();

  const handleToggle = useCallback(() => {
    onOpenChange(!open);
  }, [onOpenChange, open]);

  useEffect(() => {
    if (!open) return;

    const frame = window.requestAnimationFrame(() => {
      inputRef.current?.focus();
    });

    return () => window.cancelAnimationFrame(frame);
  }, [open]);

  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onOpenChange(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [open, onOpenChange]);

  useEffect(() => {
    if (!open) return;

    const handlePointerDown = (event: PointerEvent) => {
      if (
        rootRef.current &&
        !rootRef.current.contains(event.target as Node)
      ) {
        onOpenChange(false);
      }
    };

    document.addEventListener("pointerdown", handlePointerDown);
    return () => document.removeEventListener("pointerdown", handlePointerDown);
  }, [open, onOpenChange]);

  const panelTransition = reducedMotion
    ? { duration: 0 }
    : {
        duration: open ? MOTION_DURATION.base : MOTION_DURATION.fast,
        ease: MOTION_EASE.enter,
      };

  const fieldTransition = reducedMotion
    ? { duration: 0 }
    : {
        duration: open ? MOTION_DURATION.base : MOTION_DURATION.fast,
        ease: MOTION_EASE.enter,
      };

  return (
    <div ref={rootRef} className="inline-flex items-center">
      <button
        type="button"
        aria-label={label}
        aria-expanded={open}
        aria-controls={inputId}
        disabled={disabled}
        onClick={handleToggle}
        className={[
          "inline-flex size-9 shrink-0 items-center justify-center rounded-md border",
          "transition-colors duration-150",
          "disabled:cursor-not-allowed disabled:opacity-40",
          open
            ? "border-hairline-strong bg-surface-elevated text-ink"
            : "border-transparent text-charcoal hover:border-hairline-strong hover:bg-surface-elevated hover:text-ink",
        ].join(" ")}
      >
        <Search size={16} strokeWidth={1.75} aria-hidden />
      </button>

      <motion.div
        className="overflow-hidden"
        initial={false}
        animate={{
          width: open ? EXPANDED_WIDTH_PX : 0,
          opacity: open ? 1 : 0,
        }}
        transition={panelTransition}
        aria-hidden={!open}
      >
        <motion.div
          className="flex items-center"
          style={{ width: EXPANDED_WIDTH_PX }}
          initial={false}
          animate={{
            x: open ? 0 : -8,
          }}
          transition={fieldTransition}
        >
          <label htmlFor={inputId} className="sr-only">
            {label}
          </label>
          <input
            ref={inputRef}
            id={inputId}
            type="search"
            value={value}
            disabled={disabled}
            placeholder={placeholder}
            onChange={(event: ChangeEvent<HTMLInputElement>) => {
              onChange(event.target.value);
            }}
            className={[
              "h-9 w-full bg-transparent pl-2 pr-1",
              "font-[family-name:var(--font-ui)] text-sm leading-[1.43] text-ink",
              "placeholder:text-mute",
              "border-0 outline-none",
              "disabled:cursor-not-allowed disabled:opacity-40",
            ].join(" ")}
          />
        </motion.div>
      </motion.div>
    </div>
  );
}
