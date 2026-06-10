import type { HTMLAttributes, ReactNode } from "react";

interface CodeWindowProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
  showTrafficLights?: boolean;
}

export function CodeWindow({
  children,
  showTrafficLights = true,
  className = "",
  ...props
}: CodeWindowProps) {
  return (
    <div
      className={[
        "overflow-hidden rounded-lg border border-hairline-strong",
        "bg-surface-deep text-body",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      {...props}
    >
      {showTrafficLights && (
        <div className="flex items-center gap-1.5 border-b border-hairline px-6 py-3">
          <span className="size-2.5 rounded-full bg-accent-red" aria-hidden="true" />
          <span className="size-2.5 rounded-full bg-accent-yellow" aria-hidden="true" />
          <span className="size-2.5 rounded-full bg-accent-green" aria-hidden="true" />
        </div>
      )}
      <div className="p-6 text-code-md">{children}</div>
    </div>
  );
}
