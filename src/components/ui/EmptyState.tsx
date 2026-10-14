import React from "react";

export interface EmptyStateProps {
  icon: React.ReactNode;
  title: string;
  description: string;
  badge?: string;
  action?: React.ReactNode;
  className?: string;
}

export function EmptyState({
  icon,
  title,
  description,
  badge,
  action,
  className = "",
}: EmptyStateProps): React.ReactElement {
  return (
    <div
      className={`flex flex-col items-center justify-center p-6 text-center select-none ${className}`}
    >
      <div className="flex items-center justify-center w-12 h-12 rounded-xl bg-zinc-800/40 border border-zinc-700/40 text-zinc-400 mb-3.5 shadow-inner">
        {icon}
      </div>

      {badge && (
        <span className="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-medium bg-zinc-800/80 text-zinc-300 border border-zinc-700/40 mb-2">
          {badge}
        </span>
      )}

      <h3 className="text-sm font-semibold text-zinc-200 tracking-tight mb-1">
        {title}
      </h3>

      <p className="text-xs text-zinc-400 max-w-xs leading-relaxed mb-4">
        {description}
      </p>

      {action && <div className="mt-1">{action}</div>}
    </div>
  );
}
