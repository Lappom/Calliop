import type { InputHTMLAttributes } from "react";

interface TextInputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
}

export function TextInput({
  label,
  className = "",
  id,
  ...props
}: TextInputProps) {
  const inputId = id ?? label?.toLowerCase().replace(/\s+/g, "-");

  return (
    <div className="flex flex-col gap-2">
      {label && (
        <label
          htmlFor={inputId}
          className="text-body-sm text-charcoal"
        >
          {label}
        </label>
      )}
      <input
        id={inputId}
        className={[
          "h-10 w-full rounded-md border border-hairline-strong",
          "bg-surface-card px-3.5 py-2.5 text-ink",
          "font-[family-name:var(--font-ui)] text-sm leading-[1.43]",
          "placeholder:text-mute",
          "focus:border-ink focus:outline-none",
          "disabled:cursor-not-allowed disabled:opacity-40",
          className,
        ]
          .filter(Boolean)
          .join(" ")}
        {...props}
      />
    </div>
  );
}
