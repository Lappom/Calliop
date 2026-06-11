import type { ReactNode } from "react";
import type { ToolbarMenuOption } from "../ui/toolbarMenu";

export function SnippetListToolbarButton<T extends string = string>({
  label,
  active = false,
  disabled,
  onClick,
  children,
  menuTitle,
  menuOptions,
  activeMenuValue,
  onMenuSelect,
}: {
  label: string;
  active?: boolean;
  disabled?: boolean;
  onClick: () => void;
  children: ReactNode;
  menuTitle?: string;
  menuOptions?: ToolbarMenuOption<T>[];
  activeMenuValue?: T;
  onMenuSelect?: (value: T) => void;
}) {
  const button = (
    <button
      type="button"
      aria-label={label}
      aria-pressed={active}
      aria-haspopup={menuOptions ? "menu" : undefined}
      disabled={disabled}
      onClick={onClick}
      className={[
        "inline-flex size-9 items-center justify-center rounded-md border transition-colors duration-150",
        "disabled:cursor-not-allowed disabled:opacity-40",
        active
          ? "border-hairline-strong bg-surface-elevated text-ink"
          : "border-transparent text-charcoal hover:border-hairline-strong hover:bg-surface-elevated hover:text-ink",
      ].join(" ")}
    >
      {children}
    </button>
  );

  if (!menuOptions || menuOptions.length === 0) {
    return button;
  }

  return (
    <span className="group/toolbar-menu relative inline-flex">
      {button}
      {/* pt-2 bridges the gap so hover stays active while moving to the menu */}
      <div
        role="menu"
        aria-label={menuTitle ?? label}
        className={[
          "absolute right-0 top-full z-50 pt-2",
          "opacity-0 transition-opacity duration-150",
          "pointer-events-none group-hover/toolbar-menu:pointer-events-auto",
          "group-hover/toolbar-menu:opacity-100",
        ].join(" ")}
      >
        <div
          className={[
            "min-w-[11rem] rounded-md border border-hairline-strong",
            "bg-surface-elevated py-2 shadow-none",
          ].join(" ")}
        >
          {menuTitle && (
            <p className="text-caption m-0 px-3 pb-1.5 font-medium text-ash">
              {menuTitle}
            </p>
          )}
          <ul className="m-0 list-none p-0">
            {menuOptions.map((option) => {
              const isActive = option.value === activeMenuValue;
              return (
                <li key={option.value} role="none">
                  <button
                    type="button"
                    role="menuitemradio"
                    aria-checked={isActive}
                    disabled={disabled}
                    onClick={() => onMenuSelect?.(option.value)}
                    className={[
                      "flex w-full items-center gap-2 px-3 py-1.5 text-left",
                      "text-caption transition-colors duration-150",
                      "hover:bg-surface-card hover:text-ink",
                      "disabled:cursor-not-allowed disabled:opacity-40",
                      isActive ? "text-ink" : "text-charcoal",
                    ].join(" ")}
                  >
                    <span
                      className={[
                        "inline-block size-1.5 shrink-0 rounded-full",
                        isActive ? "bg-accent-blue" : "bg-transparent",
                      ].join(" ")}
                      aria-hidden
                    />
                    {option.label}
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      </div>
    </span>
  );
}
