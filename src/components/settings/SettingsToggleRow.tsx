import { memo, type ReactNode } from "react";
import { Toggle } from "../ui/Toggle";

interface SettingsToggleRowProps {
  label: ReactNode;
  description?: ReactNode;
  checked: boolean;
  disabled?: boolean;
  id?: string;
  onCheckedChange: (checked: boolean) => void;
}

export const SettingsToggleRow = memo(function SettingsToggleRow({
  label,
  description,
  checked,
  disabled,
  id,
  onCheckedChange,
}: SettingsToggleRowProps) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div className="min-w-0 flex-1 space-y-1">
        <p className="text-body-md text-ink">{label}</p>
        {description && (
          <p className="text-caption text-ash transition-opacity duration-200">
            {description}
          </p>
        )}
      </div>
      <Toggle
        id={id}
        checked={checked}
        disabled={disabled}
        aria-label={typeof label === "string" ? label : undefined}
        onCheckedChange={onCheckedChange}
      />
    </div>
  );
});
