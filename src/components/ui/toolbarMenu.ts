export interface ToolbarMenuOption<T extends string = string> {
  value: T;
  label: string;
}

export function toolbarMenuOptions<T extends string>(
  labels: Record<T, string>,
  order: readonly T[],
): ToolbarMenuOption<T>[] {
  return order.map((value) => ({
    value,
    label: labels[value],
  }));
}
