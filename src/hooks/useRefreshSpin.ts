import { useCallback, useRef, useState } from "react";

const MIN_REFRESH_SPIN_MS = 600;

export function useRefreshSpin(externalBusy = false) {
  const [spinning, setSpinning] = useState(false);
  const startMsRef = useRef(0);

  const runRefresh = useCallback(async (action: () => void | Promise<void>) => {
    startMsRef.current = Date.now();
    setSpinning(true);
    try {
      await action();
    } finally {
      const elapsed = Date.now() - startMsRef.current;
      const remaining = Math.max(0, MIN_REFRESH_SPIN_MS - elapsed);
      if (remaining > 0) {
        await new Promise((resolve) => window.setTimeout(resolve, remaining));
      }
      setSpinning(false);
    }
  }, []);

  return {
    spinning: spinning || externalBusy,
    runRefresh,
  };
}
