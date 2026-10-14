import React from "react";

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "subtle" | "danger";
  size?: "xs" | "sm" | "md";
  icon?: React.ReactNode;
}

export function Button({
  children,
  variant = "secondary",
  size = "sm",
  icon,
  className = "",
  disabled,
  ...props
}: ButtonProps): React.ReactElement {
  const baseStyles =
    "inline-flex items-center justify-center font-medium rounded-lg transition-colors select-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500 focus-visible:ring-offset-1 focus-visible:ring-offset-zinc-950";

  const sizeStyles = {
    xs: "text-xs px-2.5 py-1 gap-1.5 h-7",
    sm: "text-xs px-3 py-1.5 gap-2 h-8",
    md: "text-sm px-4 py-2 gap-2.5 h-9",
  }[size];

  const variantStyles = {
    primary:
      "bg-indigo-600 hover:bg-indigo-500 active:bg-indigo-700 text-white shadow-sm shadow-indigo-600/30 disabled:bg-zinc-800 disabled:text-zinc-500 disabled:shadow-none",
    secondary:
      "bg-zinc-800/80 hover:bg-zinc-700/80 active:bg-zinc-800 text-zinc-200 border border-zinc-700/60 disabled:bg-zinc-900/40 disabled:text-zinc-600 disabled:border-zinc-800/40",
    subtle:
      "bg-transparent hover:bg-zinc-800/60 active:bg-zinc-800 text-zinc-300 hover:text-zinc-100 disabled:text-zinc-600",
    danger:
      "bg-rose-600/90 hover:bg-rose-600 active:bg-rose-700 text-white disabled:bg-zinc-800 disabled:text-zinc-500",
  }[variant];

  const disabledStyles = disabled ? "cursor-not-allowed opacity-60" : "cursor-pointer";

  return (
    <button
      className={`${baseStyles} ${sizeStyles} ${variantStyles} ${disabledStyles} ${className}`}
      disabled={disabled}
      {...props}
    >
      {icon && <span className="inline-flex shrink-0">{icon}</span>}
      {children}
    </button>
  );
}
