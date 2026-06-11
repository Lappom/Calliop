import type { ReactNode } from "react";
import { sectionGlowClasses } from "./glowSurface";

type GlowColor = "green" | "blue" | "red" | "orange" | "none";

interface SectionGlowProps {
  children: ReactNode;
  glow?: GlowColor;
  className?: string;
}

export function SectionGlow({
  children,
  glow = "none",
  className = "",
}: SectionGlowProps) {
  return (
    <section
      className={[
        glow !== "none" ? sectionGlowClasses(glow) : "relative",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {children}
    </section>
  );
}
