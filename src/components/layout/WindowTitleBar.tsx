import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauri } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState, type ReactNode } from "react";

function MinimizeIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden>
      <path d="M0 5h10" stroke="currentColor" strokeWidth="1" />
    </svg>
  );
}

function MaximizeIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden>
      <rect
        x="0.5"
        y="0.5"
        width="9"
        height="9"
        fill="none"
        stroke="currentColor"
        strokeWidth="1"
      />
    </svg>
  );
}

function RestoreIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden>
      <rect
        x="2.5"
        y="0.5"
        width="7"
        height="7"
        fill="none"
        stroke="currentColor"
        strokeWidth="1"
      />
      <path
        d="M0.5 2.5v7h7"
        fill="none"
        stroke="currentColor"
        strokeWidth="1"
      />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden>
      <path d="M1 1l8 8M9 1L1 9" stroke="currentColor" strokeWidth="1" />
    </svg>
  );
}

interface WindowControlProps {
  label: string;
  onClick: () => void;
  variant?: "default" | "close";
  children: ReactNode;
}

function WindowControl({
  label,
  onClick,
  variant = "default",
  children,
}: WindowControlProps) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className={[
        "inline-flex h-8 w-[46px] shrink-0 items-center justify-center text-ink/90 transition-colors",
        variant === "close"
          ? "hover:bg-[#c42b1c] hover:text-white active:bg-[#b22a1a]"
          : "hover:bg-white/10 active:bg-white/15",
      ].join(" ")}
    >
      {children}
    </button>
  );
}

export function WindowTitleBar() {
  const [maximized, setMaximized] = useState(false);

  const syncMaximized = useCallback(async () => {
    if (!isTauri()) {
      return;
    }
    const win = getCurrentWindow();
    setMaximized(await win.isMaximized());
  }, []);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    void syncMaximized();
    const win = getCurrentWindow();
    const unlisten = win.onResized(() => {
      void syncMaximized();
    });

    return () => {
      void unlisten.then((drop) => drop());
    };
  }, [syncMaximized]);

  if (!isTauri()) {
    return null;
  }

  const win = getCurrentWindow();

  return (
    <header className="flex h-8 shrink-0 select-none border-b border-hairline bg-canvas">
      <div className="flex min-w-0 flex-1 items-center gap-2 px-3">
        <div
          data-tauri-drag-region
          className="flex min-w-0 flex-1 items-center gap-2"
          onDoubleClick={() => {
            void win.toggleMaximize();
          }}
        >
          <img
            src="/app-icon.png"
            alt=""
            width={16}
            height={16}
            draggable={false}
            className="size-4 shrink-0"
          />
          <span className="text-display-serif text-sm leading-none text-ink">
            Calliop
          </span>
        </div>
      </div>

      <div className="flex h-full shrink-0">
        <WindowControl
          label="Réduire"
          onClick={() => {
            void win.minimize();
          }}
        >
          <MinimizeIcon />
        </WindowControl>
        <WindowControl
          label={maximized ? "Restaurer" : "Agrandir"}
          onClick={() => {
            void win.toggleMaximize();
          }}
        >
          {maximized ? <RestoreIcon /> : <MaximizeIcon />}
        </WindowControl>
        <WindowControl
          label="Fermer"
          variant="close"
          onClick={() => {
            void win.close();
          }}
        >
          <CloseIcon />
        </WindowControl>
      </div>
    </header>
  );
}
