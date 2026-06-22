import {
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { CircleHelp } from "lucide-react";

interface TooltipProps {
  content: string;
  children?: ReactNode;
  className?: string;
}

const SHOW_DELAY_MS = 300;

let adjacentTooltipActive = false;

export function Tooltip({ content, children, className = "" }: TooltipProps) {
  const tooltipId = useId();
  const showTimerRef = useRef<number | null>(null);
  const adjacentActiveRef = useRef(false);
  const [visible, setVisible] = useState(false);
  const [instant, setInstant] = useState(false);

  const clearShowTimer = useCallback(() => {
    if (showTimerRef.current != null) {
      window.clearTimeout(showTimerRef.current);
      showTimerRef.current = null;
    }
  }, []);

  const show = useCallback(
    (immediate: boolean) => {
      clearShowTimer();
      if (immediate || adjacentTooltipActive) {
        setInstant(true);
        setVisible(true);
        adjacentTooltipActive = true;
        adjacentActiveRef.current = true;
        return;
      }
      showTimerRef.current = window.setTimeout(() => {
        setInstant(false);
        setVisible(true);
        adjacentTooltipActive = true;
        adjacentActiveRef.current = true;
      }, SHOW_DELAY_MS);
    },
    [clearShowTimer],
  );

  const hide = useCallback(() => {
    clearShowTimer();
    setVisible(false);
    setInstant(false);
    adjacentTooltipActive = false;
    adjacentActiveRef.current = false;
  }, [clearShowTimer]);

  useEffect(() => {
    return () => {
      clearShowTimer();
      if (adjacentActiveRef.current) {
        adjacentTooltipActive = false;
      }
    };
  }, [clearShowTimer]);

  return (
    <span
      className={[
        "group/tooltip relative inline-flex shrink-0 align-middle",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      onPointerEnter={() => show(false)}
      onPointerLeave={hide}
      onFocus={() => show(true)}
      onBlur={hide}
    >
      <span
        tabIndex={0}
        aria-describedby={visible ? tooltipId : undefined}
        className={[
          "inline-flex cursor-default rounded-sm text-ash outline-none",
          "transition-colors duration-150",
          "hover:text-charcoal focus-visible:text-charcoal",
        ].join(" ")}
      >
        {children ?? <CircleHelp size={15} strokeWidth={1.75} aria-hidden />}
        <span className="sr-only">{content}</span>
      </span>
      <span
        id={tooltipId}
        role="tooltip"
        className={[
          "tooltip-bubble pointer-events-none absolute left-1/2 top-full z-50 mt-2 w-56",
          "rounded-md border border-hairline-strong bg-surface-elevated px-3 py-2",
          "text-caption leading-snug text-charcoal",
          visible ? "tooltip-bubble-visible" : "",
          instant ? "tooltip-bubble-instant" : "",
        ].join(" ")}
      >
        {content}
      </span>
    </span>
  );
}

/** @deprecated Use Tooltip — kept for existing imports */
export function InfoTooltip(props: TooltipProps) {
  return <Tooltip {...props} />;
}
