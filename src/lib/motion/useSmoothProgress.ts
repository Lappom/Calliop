import { useEffect, useRef, useState } from "react";

/** Per-frame easing toward the target (~250ms time constant at 60fps). */
const SMOOTHING = 0.14;
const SNAP_THRESHOLD = 0.02;

/**
 * Interpolates discrete progress updates into a continuous display value.
 * No overshoot — suitable for download progress bars.
 */
export function useSmoothProgress(target: number, instant = false): number {
  const [display, setDisplay] = useState(target);
  const targetRef = useRef(target);
  const displayRef = useRef(target);

  targetRef.current = target;

  useEffect(() => {
    if (instant) {
      displayRef.current = targetRef.current;
      setDisplay(targetRef.current);
      return;
    }

    let frame = 0;

    const step = () => {
      const goal = targetRef.current;
      let current = displayRef.current;
      const delta = goal - current;

      if (Math.abs(delta) <= SNAP_THRESHOLD) {
        if (current !== goal) {
          displayRef.current = goal;
          setDisplay(goal);
        }
      } else {
        current += delta * SMOOTHING;
        displayRef.current = current;
        setDisplay(current);
      }

      frame = requestAnimationFrame(step);
    };

    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  }, [instant]);

  useEffect(() => {
    if (instant) {
      displayRef.current = target;
      setDisplay(target);
    }
  }, [target, instant]);

  return display;
}
