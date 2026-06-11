import {
  useEffect,
  useState,
  type InputHTMLAttributes,
} from "react";

interface ToggleProps
  extends Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "onChange"> {
  onCheckedChange?: (checked: boolean) => void;
}

export function Toggle({
  checked = false,
  disabled,
  id,
  className = "",
  onCheckedChange,
  "aria-label": ariaLabel,
  ...props
}: ToggleProps) {
  const [isOn, setIsOn] = useState(checked);

  useEffect(() => {
    setIsOn(checked);
  }, [checked]);

  return (
    <label
      className={[
        "relative inline-flex shrink-0 cursor-pointer items-center",
        disabled ? "cursor-not-allowed opacity-40" : "",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <input
        type="checkbox"
        role="switch"
        id={id}
        checked={isOn}
        disabled={disabled}
        aria-label={ariaLabel}
        aria-checked={isOn}
        onChange={(event) => {
          const next = event.target.checked;
          setIsOn(next);
          onCheckedChange?.(next);
        }}
        className="peer sr-only"
        {...props}
      />
      <span
        aria-hidden
        className={[
          "relative inline-flex h-6 w-11 items-center rounded-full border p-0.5",
          "transition-[background-color,border-color,box-shadow] duration-300 ease-out",
          isOn
            ? "border-accent-blue bg-accent-blue shadow-[0_0_12px_var(--color-accent-blue-glow)]"
            : "border-hairline-strong bg-surface-elevated",
          "peer-focus-visible:outline peer-focus-visible:outline-1 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-ink",
        ].join(" ")}
      >
        <span
          className={[
            "block size-5 shrink-0 rounded-full",
            "transition-transform duration-300 ease-[cubic-bezier(0.34,1.3,0.64,1)]",
            "will-change-transform",
            isOn
              ? "translate-x-5 bg-primary"
              : "translate-x-0 bg-charcoal",
          ].join(" ")}
        />
      </span>
    </label>
  );
}
