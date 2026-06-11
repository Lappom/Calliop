import { Check, ChevronDown, CircleCheck, CloudOff, HardDrive } from "lucide-react";
import {
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { createPortal } from "react-dom";

export interface SelectOption<T extends string = string> {
  value: T;
  label: string;
  status?: "active" | "installed" | "missing";
  statusLabel?: string;
}

function OptionStatusIcon({
  status,
  statusLabel,
  className = "",
}: {
  status?: SelectOption["status"];
  statusLabel?: string;
  className?: string;
}) {
  if (!status) return null;

  const base = ["shrink-0", className].filter(Boolean).join(" ");

  const icon =
    status === "active" ? (
      <CircleCheck
        size={15}
        strokeWidth={2}
        className={[base, "text-accent-green"].join(" ")}
        aria-hidden
      />
    ) : status === "installed" ? (
      <HardDrive
        size={15}
        strokeWidth={2}
        className={[base, "text-charcoal"].join(" ")}
        aria-hidden
      />
    ) : (
      <CloudOff
        size={15}
        strokeWidth={2}
        className={[base, "text-ash"].join(" ")}
        aria-hidden
      />
    );

  if (!statusLabel) return icon;

  return (
    <span className="inline-flex shrink-0" title={statusLabel}>
      {icon}
    </span>
  );
}

interface SelectProps<T extends string> {
  id?: string;
  label?: string;
  value: T;
  options: SelectOption<T>[];
  onChange: (value: T) => void;
  disabled?: boolean;
  className?: string;
}

export function Select<T extends string>({
  id,
  label,
  value,
  options,
  onChange,
  disabled = false,
  className = "",
}: SelectProps<T>) {
  const generatedId = useId();
  const selectId = id ?? generatedId;
  const listboxId = `${selectId}-listbox`;

  const [open, setOpen] = useState(false);
  const [highlightedIndex, setHighlightedIndex] = useState(-1);
  const [panelStyle, setPanelStyle] = useState({ top: 0, left: 0, width: 0 });

  const triggerRef = useRef<HTMLButtonElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  const selectedIndex = options.findIndex((option) => option.value === value);
  const selectedOption =
    selectedIndex >= 0 ? options[selectedIndex] : options[0];

  const statusToneClass = (status?: SelectOption<T>["status"]) => {
    switch (status) {
      case "active":
        return "text-accent-green";
      case "installed":
        return "text-charcoal";
      default:
        return "text-ash";
    }
  };

  const updatePanelPosition = useCallback(() => {
    const trigger = triggerRef.current;
    if (!trigger) return;

    const rect = trigger.getBoundingClientRect();
    setPanelStyle({
      top: rect.bottom + 6,
      left: rect.left,
      width: rect.width,
    });
  }, []);

  const close = useCallback(() => {
    setOpen(false);
    setHighlightedIndex(-1);
  }, []);

  const openList = useCallback(() => {
    if (disabled) return;
    setOpen(true);
    setHighlightedIndex(selectedIndex >= 0 ? selectedIndex : 0);
    updatePanelPosition();
  }, [disabled, selectedIndex, updatePanelPosition]);

  const selectOption = useCallback(
    (index: number) => {
      const option = options[index];
      if (!option) return;
      onChange(option.value);
      close();
      triggerRef.current?.focus();
    },
    [close, onChange, options],
  );

  useEffect(() => {
    if (!open) return;
    panelRef.current?.focus();
  }, [open]);

  useEffect(() => {
    if (!open) return;

    updatePanelPosition();

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
      if (
        triggerRef.current?.contains(target) ||
        panelRef.current?.contains(target)
      ) {
        return;
      }
      close();
    };

    document.addEventListener("mousedown", handlePointerDown);
    return () => document.removeEventListener("mousedown", handlePointerDown);
  }, [close, open]);

  const handleTriggerKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (disabled) return;

    switch (event.key) {
      case "ArrowDown":
      case "ArrowUp":
      case "Enter":
      case " ":
        event.preventDefault();
        if (!open) {
          openList();
        } else if (event.key === "ArrowDown") {
          setHighlightedIndex((current) =>
            Math.min(current + 1, options.length - 1),
          );
        } else if (event.key === "ArrowUp") {
          setHighlightedIndex((current) => Math.max(current - 1, 0));
        } else if (
          (event.key === "Enter" || event.key === " ") &&
          highlightedIndex >= 0
        ) {
          selectOption(highlightedIndex);
        }
        break;
      case "Escape":
        if (open) {
          event.preventDefault();
          close();
        }
        break;
      default:
        break;
    }
  };

  const handlePanelKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        setHighlightedIndex((current) =>
          Math.min(current + 1, options.length - 1),
        );
        break;
      case "ArrowUp":
        event.preventDefault();
        setHighlightedIndex((current) => Math.max(current - 1, 0));
        break;
      case "Enter":
      case " ":
        event.preventDefault();
        if (highlightedIndex >= 0) {
          selectOption(highlightedIndex);
        }
        break;
      case "Escape":
        event.preventDefault();
        close();
        triggerRef.current?.focus();
        break;
      case "Tab":
        close();
        break;
      default:
        break;
    }
  };

  return (
    <div className={["flex flex-col gap-2", className].filter(Boolean).join(" ")}>
      {label && (
        <label htmlFor={selectId} className="text-body-sm text-charcoal">
          {label}
        </label>
      )}

      <button
        ref={triggerRef}
        id={selectId}
        type="button"
        role="combobox"
        aria-controls={listboxId}
        aria-expanded={open}
        aria-haspopup="listbox"
        disabled={disabled}
        onClick={() => (open ? close() : openList())}
        onKeyDown={handleTriggerKeyDown}
        className={[
          "flex h-10 w-full items-center justify-between gap-3 rounded-md border px-3.5",
          "font-[family-name:var(--font-ui)] text-sm leading-[1.43] text-ink",
          "transition-[border-color,background-color] duration-150",
          open
            ? "border-ink bg-surface-elevated"
            : "border-hairline-strong bg-surface-card hover:border-ink/30",
          "focus-visible:border-ink focus-visible:outline-none",
          "disabled:cursor-not-allowed disabled:opacity-40",
        ].join(" ")}
      >
        <span className="flex min-w-0 flex-1 items-center gap-2.5 truncate text-left">
          <OptionStatusIcon
            status={selectedOption?.status}
            statusLabel={selectedOption?.statusLabel}
          />
          <span className="truncate">{selectedOption?.label ?? ""}</span>
        </span>
        <ChevronDown
          size={16}
          className={[
            "shrink-0 text-charcoal transition-transform duration-150",
            open ? "rotate-180 text-ink" : "",
          ].join(" ")}
          aria-hidden
        />
      </button>

      {open &&
        createPortal(
          <div
            ref={panelRef}
            id={listboxId}
            role="listbox"
            tabIndex={-1}
            aria-labelledby={selectId}
            onKeyDown={handlePanelKeyDown}
            style={{
              position: "fixed",
              top: panelStyle.top,
              left: panelStyle.left,
              width: panelStyle.width,
              zIndex: 50,
            }}
            className={[
              "overflow-hidden rounded-lg border border-hairline-strong bg-surface-elevated p-1 shadow-none",
              "animate-[select-panel-in_120ms_ease-out]",
            ].join(" ")}
          >
            <ul className="relative m-0 max-h-60 list-none overflow-y-auto p-0">
              {options.map((option, index) => {
                const isSelected = option.value === value;
                const isHighlighted = index === highlightedIndex;

                return (
                  <li key={option.value} role="presentation">
                    <button
                      type="button"
                      role="option"
                      aria-selected={isSelected}
                      aria-label={
                        option.statusLabel
                          ? `${option.label}, ${option.statusLabel}`
                          : option.label
                      }
                      onMouseEnter={() => setHighlightedIndex(index)}
                      onClick={() => selectOption(index)}
                      className={[
                        "flex w-full items-center gap-2.5 rounded-md px-3 py-2.5 text-left",
                        "font-[family-name:var(--font-ui)] text-sm leading-[1.43]",
                        "transition-colors duration-100",
                        isSelected
                          ? "bg-surface-card text-ink"
                          : "text-body",
                        isHighlighted && !isSelected
                          ? "bg-surface-card/70 text-ink"
                          : "",
                        !isSelected && !isHighlighted
                          ? "hover:bg-surface-card/70 hover:text-ink"
                          : "",
                      ].join(" ")}
                    >
                      <OptionStatusIcon
                        status={option.status}
                        statusLabel={option.statusLabel}
                      />
                      <span className="min-w-0 flex-1 truncate">
                        {option.label}
                      </span>
                      <span className="flex shrink-0 items-center gap-2">
                        {option.statusLabel && (
                          <span
                            className={[
                              "text-caption",
                              statusToneClass(option.status),
                            ].join(" ")}
                          >
                            {option.statusLabel}
                          </span>
                        )}
                        {isSelected && (
                          <Check
                            size={14}
                            strokeWidth={2.5}
                            className="text-accent-blue"
                            aria-hidden
                          />
                        )}
                      </span>
                    </button>
                  </li>
                );
              })}
            </ul>
          </div>,
          document.body,
        )}
    </div>
  );
}
