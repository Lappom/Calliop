import type { ReactNode } from "react";

interface ChartFrameProps {
  children: ReactNode;
  legend?: ReactNode;
  ariaLabel: string;
}

export function ChartFrame({ children, legend, ariaLabel }: ChartFrameProps) {
  return (
    <figure
      className="m-0 flex min-h-[248px] flex-col"
      aria-label={ariaLabel}
    >
      <div className="h-[200px] w-full shrink-0">{children}</div>
      <div className="mt-3 flex h-7 shrink-0 items-center">{legend}</div>
    </figure>
  );
}
