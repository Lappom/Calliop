type GlowColor = "green" | "blue" | "red" | "orange";

const glowStyles: Record<GlowColor, string> = {
  green:
    "before:bg-[radial-gradient(ellipse_80%_50%_at_50%_-20%,var(--color-accent-green-glow),transparent_70%)]",
  blue:
    "before:bg-[radial-gradient(ellipse_80%_50%_at_50%_-20%,var(--color-accent-blue-glow),transparent_70%)]",
  red:
    "before:bg-[radial-gradient(ellipse_80%_50%_at_50%_-20%,var(--color-accent-red-glow),transparent_70%)]",
  orange:
    "before:bg-[radial-gradient(ellipse_80%_50%_at_50%_-20%,var(--color-accent-orange-glow),transparent_70%)]",
};

const glowBaseClasses =
  "relative overflow-hidden before:pointer-events-none before:absolute before:inset-x-0 before:top-0 before:h-28 before:content-['']";

export function glowSurfaceClasses(glow: GlowColor): string {
  return [glowBaseClasses, glowStyles[glow]].join(" ");
}

export function sectionGlowClasses(glow: GlowColor): string {
  return [
    "relative",
    "before:pointer-events-none before:absolute before:inset-x-0 before:top-0 before:h-48 before:content-['']",
    glowStyles[glow],
  ].join(" ");
}
