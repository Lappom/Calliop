import type { ReactNode } from "react";
import { WindowTitleBar } from "./WindowTitleBar";

interface AppFrameProps {
  children: ReactNode;
}

export function AppFrame({ children }: AppFrameProps) {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-canvas text-body">
      <WindowTitleBar />
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden">{children}</div>
    </div>
  );
}
