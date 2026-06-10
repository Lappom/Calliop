import type { ReactNode } from "react";

type GlowColor = "green" | "blue" | "red" | "orange" | "none";

interface SectionGlowProps {
  children: ReactNode;
  glow?: GlowColor;
  className?: string;
}

const glowStyles: Record<Exclude<GlowColor, "none">, string> = {
  green:
    "before:bg-[radial-gradient(ellipse_80%_50%_at_50%_-20%,var(--color-accent-green-glow),transparent_70%)]",
  blue:
    "before:bg-[radial-gradient(ellipse_80%_50%_at_50%_-20%,var(--color-accent-blue-glow),transparent_70%)]",
  red:
    "before:bg-[radial-gradient(ellipse_80%_50%_at_50%_-20%,var(--color-accent-red-glow),transparent_70%)]",
  orange:
    "before:bg-[radial-gradient(ellipse_80%_50%_at_50%_-20%,var(--color-accent-orange-glow),transparent_70%)]",
};

export function SectionGlow({
  children,
  glow = "none",
  className = "",
}: SectionGlowProps) {
  return (
    <section
      className={[
        "relative",
        glow !== "none" &&
          "before:pointer-events-none before:absolute before:inset-x-0 before:top-0 before:h-48 before:content-['']",
        glow !== "none" ? glowStyles[glow] : "",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {children}
    </section>
  );
}
