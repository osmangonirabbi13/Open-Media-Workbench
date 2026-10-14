import React, { useRef } from "react";
import type { WorkspaceId } from "../../types/workspace";
import { WORKSPACE_DEFINITIONS } from "../../types/workspace";
import { WrenchIcon, GlobeIcon } from "../ui/Icons";

export interface WorkspaceSwitcherProps {
  activeWorkspace: WorkspaceId;
  onSelectWorkspace: (workspace: WorkspaceId) => void;
  className?: string;
}

export function WorkspaceSwitcher({
  activeWorkspace,
  onSelectWorkspace,
  className = "",
}: WorkspaceSwitcherProps): React.ReactElement {
  const tabsRef = useRef<(HTMLButtonElement | null)[]>([]);

  const workspaces: { id: WorkspaceId; icon: React.ReactElement; shortcut: string }[] = [
    {
      id: "repair",
      icon: <WrenchIcon size={14} className="shrink-0" />,
      shortcut: "Alt+1",
    },
    {
      id: "localization",
      icon: <GlobeIcon size={14} className="shrink-0" />,
      shortcut: "Alt+2",
    },
  ];

  const handleKeyDown = (e: React.KeyboardEvent<HTMLButtonElement>, currentIndex: number) => {
    let targetIndex: number | null = null;

    if (e.key === "ArrowRight" || e.key === "ArrowDown") {
      e.preventDefault();
      targetIndex = (currentIndex + 1) % workspaces.length;
    } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
      e.preventDefault();
      targetIndex = (currentIndex - 1 + workspaces.length) % workspaces.length;
    } else if (e.key === "Home") {
      e.preventDefault();
      targetIndex = 0;
    } else if (e.key === "End") {
      e.preventDefault();
      targetIndex = workspaces.length - 1;
    }

    if (targetIndex !== null) {
      const nextWorkspace = workspaces[targetIndex].id;
      onSelectWorkspace(nextWorkspace);
      tabsRef.current[targetIndex]?.focus();
    }
  };

  return (
    <nav
      className={`flex items-center ${className}`}
      aria-label="Workspace Selection"
    >
      <div
        role="tablist"
        aria-label="Studio Workspaces"
        className="flex items-center p-0.5 rounded-lg bg-zinc-900/90 border border-zinc-800/80 shadow-xs"
      >
        {workspaces.map((ws, index) => {
          const def = WORKSPACE_DEFINITIONS[ws.id];
          const isActive = activeWorkspace === ws.id;

          return (
            <button
              key={ws.id}
              ref={(el) => {
                tabsRef.current[index] = el;
              }}
              role="tab"
              id={`workspace-tab-${ws.id}`}
              aria-controls={`workspace-panel-${ws.id}`}
              aria-selected={isActive}
              tabIndex={isActive ? 0 : -1}
              onClick={() => onSelectWorkspace(ws.id)}
              onKeyDown={(e) => handleKeyDown(e, index)}
              title={`${def.label}: ${def.description} (${ws.shortcut})`}
              className={`group relative flex items-center space-x-2 px-3 py-1 rounded-md text-xs font-medium transition-all duration-150 outline-none focus-visible:ring-2 focus-visible:ring-indigo-500 focus-visible:ring-offset-1 focus-visible:ring-offset-zinc-950 ${
                isActive
                  ? "bg-zinc-800 text-zinc-100 shadow-xs border border-zinc-700/60"
                  : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40"
              }`}
            >
              <span
                className={`transition-colors ${
                  isActive ? "text-indigo-400" : "text-zinc-400 group-hover:text-zinc-300"
                }`}
              >
                {ws.icon}
              </span>

              <span className="tracking-tight">{def.shortLabel}</span>

              {/* Status Badge */}
              <span
                className={`hidden sm:inline-block px-1.5 py-0.2 rounded text-[9px] font-mono uppercase tracking-wider ${
                  isActive
                    ? "bg-indigo-950/80 text-indigo-300 border border-indigo-800/50"
                    : "bg-zinc-800/80 text-zinc-400 border border-zinc-700/30"
                }`}
              >
                {def.badge}
              </span>

              {/* Keyboard Shortcut Hint */}
              <kbd
                className={`hidden md:inline-block text-[9px] font-mono px-1 rounded transition-opacity ${
                  isActive
                    ? "bg-zinc-900 text-zinc-400 border border-zinc-700/40"
                    : "opacity-40 group-hover:opacity-80 text-zinc-400"
                }`}
              >
                {ws.shortcut}
              </kbd>
            </button>
          );
        })}
      </div>
    </nav>
  );
}
