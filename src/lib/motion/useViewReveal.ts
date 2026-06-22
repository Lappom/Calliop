import { useEffect, useState } from "react";

const visitedViews = new Set<string>();
const visitedStaggerInstances = new Set<string>();

function staggerInstanceKey(viewKey: string, instanceId: string): string {
  return `${viewKey}::${instanceId}`;
}

/** True only on the first mount of a view — skips repeat stagger on sidebar revisit. */
export function useViewReveal(viewKey: string, instanceId?: string): boolean {
  const instanceKey =
    instanceId != null ? staggerInstanceKey(viewKey, instanceId) : viewKey;

  const [shouldReveal] = useState(() => {
    if (visitedViews.has(viewKey)) return false;
    return !visitedStaggerInstances.has(instanceKey);
  });

  useEffect(() => {
    visitedStaggerInstances.add(instanceKey);
  }, [instanceKey]);

  return shouldReveal;
}

export function markViewVisited(viewKey: string): void {
  visitedViews.add(viewKey);
}
