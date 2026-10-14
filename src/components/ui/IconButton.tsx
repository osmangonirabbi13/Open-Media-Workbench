import React from "react";

export interface IconButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  icon: React.ReactNode;
  label: string;
  size?: "xs" | "sm" | "md";
  variant?: "default" | "subtle" | "active" | "danger";
  shortcut?: string;
}

export function IconButton({
  icon,
  label,
  size = "sm",
  variant = "default",
  shortcut,
  className = "",
  disabled,
  ...props
}: IconButtonProps): React.ReactElement {
  const sizeStyles = {
    xs: "w-6 h-6 p-1",
    sm: "w-7 h-7 p-1.5",
    md: "w-8 h-8 p-2",
  }[size];

  const variantStyles = {
    default:
      "bg-zinc-800/60 hover:bg-zinc-700/80 active:bg-zinc-800 text-zinc-300 hover:text-zinc-100 border border-zinc-700/50",
    subtle:
      "bg-transparent hover:bg-zinc-800/60 active:bg-zinc-800 text-zinc-400 hover:text-zinc-200 border-transparent",
    active:
      "bg-indigo-600/20 text-indigo-400 border border-indigo-500/40 hover:bg-indigo-600/30",
    danger:
      "bg-transparent hover:bg-rose-950/40 text-zinc-400 hover:text-rose-300 border-transparent",
  }[variant];

  const disabledStyles = disabled
    ? "cursor-not-allowed opacity-40 hover:bg-transparent hover:text-zinc-400"
    : "cursor-pointer";

  const titleWithShortcut = shortcut ? `${label} (${shortcut})` : label;

  return (
    <button
      type="button"
      aria-label={label}
      title={titleWithShortcut}
      className={`relative inline-flex items-center justify-center rounded-lg border transition-colors select-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500 focus-visible:ring-offset-1 focus-visible:ring-offset-zinc-950 ${sizeStyles} ${variantStyles} ${disabledStyles} ${className}`}
      disabled={disabled}
      {...props}
    >
      {icon}
    </button>
  );
}
