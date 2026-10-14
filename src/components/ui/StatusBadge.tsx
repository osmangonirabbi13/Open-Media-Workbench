import React from "react";

export interface StatusBadgeProps {
  status: "online" | "connecting" | "offline" | "idle";
  label?: string;
  className?: string;
}

export function StatusBadge({
  status,
  label,
  className = "",
}: StatusBadgeProps): React.ReactElement {
  const configs = {
    online: {
      dot: "bg-emerald-400 shadow-sm shadow-emerald-400/50",
      text: "text-emerald-300",
      border: "border-emerald-900/40 bg-emerald-950/20",
      defaultLabel: "Rust Core Online",
    },
    connecting: {
      dot: "bg-amber-400 animate-pulse",
      text: "text-amber-300",
      border: "border-amber-900/40 bg-amber-950/20",
      defaultLabel: "Connecting...",
    },
    offline: {
      dot: "bg-rose-400 shadow-sm shadow-rose-400/50",
      text: "text-rose-300",
      border: "border-rose-900/40 bg-rose-950/20",
      defaultLabel: "Core Offline",
    },
    idle: {
      dot: "bg-zinc-500",
      text: "text-zinc-400",
      border: "border-zinc-800 bg-zinc-900/40",
      defaultLabel: "Initializing",
    },
  }[status];

  return (
    <div
      role="status"
      aria-live="polite"
      className={`inline-flex items-center space-x-1.5 px-2.5 py-1 rounded-full text-xs font-medium border ${configs.border} ${className}`}
    >
      <span className={`w-1.5 h-1.5 rounded-full ${configs.dot}`} aria-hidden="true" />
      <span className={configs.text}>{label || configs.defaultLabel}</span>
    </div>
  );
}
