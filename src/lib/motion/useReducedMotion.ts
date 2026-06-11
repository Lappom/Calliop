import { useEffect, useState } from "react";

const QUERY = "(prefers-reduced-motion: reduce)";

function getReducedMotionPreference(): boolean {
  if (typeof window === "undefined") {
    return false;
  }

  return window.matchMedia(QUERY).matches;
}

export function useReducedMotion(): boolean {
  const [reducedMotion, setReducedMotion] = useState(getReducedMotionPreference);

  useEffect(() => {
    const mediaQuery = window.matchMedia(QUERY);
    const onChange = () => {
      setReducedMotion(mediaQuery.matches);
    };

    mediaQuery.addEventListener("change", onChange);
    return () => mediaQuery.removeEventListener("change", onChange);
  }, []);

  return reducedMotion;
}
