import { useState, type ReactNode } from "react";
import { IconButton } from "../ui/IconButton";
import { Popover } from "../ui/Popover";
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
  const [menuOpen, setMenuOpen] = useState(false);

  const button = (
    <IconButton
      label={label}
      size="md"
      active={active || menuOpen}
      disabled={disabled}
      aria-pressed={active}
      aria-haspopup={menuOptions ? "menu" : undefined}
      aria-expanded={menuOptions ? menuOpen : undefined}
      data-popover-trigger=""
      onClick={() => {
        if (menuOptions && menuOptions.length > 0) {
          setMenuOpen((current) => {
            if (!current) onClick();
            return !current;
          });
          return;
        }
        onClick();
      }}
    >
      {children}
    </IconButton>
  );

  if (!menuOptions || menuOptions.length === 0) {
    return button;
  }

  return (
    <Popover
      open={menuOpen}
      onOpenChange={setMenuOpen}
      align="end"
      menuLabel={menuTitle ?? label}
      trigger={button}
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
                  onClick={() => {
                    onMenuSelect?.(option.value);
                    setMenuOpen(false);
                  }}
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
    </Popover>
  );
}
