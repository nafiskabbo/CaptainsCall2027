/**
 * Captain's Call 2027 — UI kit
 *
 * Small, dependency-free components that carry the Cricket Coach 2014 chrome:
 * beveled blue buttons, slate panel heads, dense tables, gold section labels.
 * Screens compose these instead of re-styling raw elements, so the look stays
 * consistent and textures/buttons are defined exactly once.
 */
import {
  type ButtonHTMLAttributes,
  type InputHTMLAttributes,
  type ReactNode,
  type SelectHTMLAttributes,
  useEffect,
  useId,
  useState,
} from "react";

/* ------------------------------------------------------------------ Button */

type ButtonVariant = "blue" | "gray" | "danger" | "continue" | "ghost";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: "sm" | "md";
  icon?: ReactNode;
}

const VARIANT_CLASS: Record<ButtonVariant, string> = {
  blue: "btn",
  gray: "btn btn--gray",
  danger: "btn btn--danger",
  continue: "btn btn--continue",
  ghost: "btn btn--ghost",
};

/** A beveled chrome button. `variant="continue"` renders the arrow pill. */
export function Button({
  variant = "blue",
  size = "md",
  icon,
  children,
  className = "",
  ...rest
}: ButtonProps) {
  const sizeClass = size === "sm" ? "btn--sm" : "";
  if (variant === "continue") {
    return (
      <button
        type="button"
        className={`${VARIANT_CLASS.continue} ${sizeClass} ${className}`.trim()}
        {...rest}
      >
        <span>{children}</span>
        <span className="btn--continue__arrow" aria-hidden="true">
          {icon ?? "➜"}
        </span>
      </button>
    );
  }
  return (
    <button
      type="button"
      className={`${VARIANT_CLASS[variant]} ${sizeClass} ${className}`.trim()}
      {...rest}
    >
      {children}
    </button>
  );
}

/* ------------------------------------------------------------------- Panel */

interface PanelProps {
  title?: string;
  /** Gold label in the head strip (the original's section labels). */
  goldTitle?: string;
  actions?: ReactNode;
  flush?: boolean;
  children: ReactNode;
  className?: string;
}

/**
 * Translucent panel over the field with the beveled slate head strip.
 * Pass `goldTitle` for the original's gold section labels.
 */
export function Panel({
  title,
  goldTitle,
  actions,
  flush,
  children,
  className = "",
}: PanelProps) {
  return (
    <section className={`panel ${flush ? "panel--flush" : ""} ${className}`.trim()}>
      {(title || goldTitle || actions) && (
        <div className="panel__head">
          <span className="panel__label--gold">{goldTitle ?? title}</span>
          {actions && <div className="panel__headactions">{actions}</div>}
        </div>
      )}
      {children}
    </section>
  );
}

/* -------------------------------------------------------------------- Tabs */

interface TabsProps<T extends string> {
  tabs: { id: T; label: string }[];
  active: T;
  onChange: (id: T) => void;
  className?: string;
}

/** Raised tab strips on a dark groove. */
export function Tabs<T extends string>({
  tabs,
  active,
  onChange,
  className = "",
}: TabsProps<T>) {
  return (
    <nav className={`tabs ${className}`.trim()} role="tablist">
      {tabs.map((t) => (
        <button
          key={t.id}
          type="button"
          role="tab"
          aria-selected={active === t.id}
          className={`tab ${active === t.id ? "tab--active" : ""}`}
          onClick={() => onChange(t.id)}
        >
          {t.label}
        </button>
      ))}
    </nav>
  );
}

/* ------------------------------------------------------------------- Table */

interface Column<T> {
  key: string;
  label: string;
  /** Right-aligned numeric column. */
  num?: boolean;
  width?: number | string;
  render: (row: T, index: number) => ReactNode;
}

interface TableProps<T> {
  columns: Column<T>[];
  rows: T[];
  rowKey: (row: T, index: number) => string;
  /** Highlight the row (used for the player's own team/fixtures). */
  rowHighlight?: (row: T) => boolean;
  emptyMessage?: string;
  maxHeight?: number;
}

/** Dense tabular table with the original's near-black rows. */
export function Table<T>({
  columns,
  rows,
  rowKey,
  rowHighlight,
  emptyMessage = "No rows.",
  maxHeight,
}: TableProps<T>) {
  return (
    <div className="tablewrap" style={maxHeight ? { maxHeight } : undefined}>
      <table className="data">
        <thead>
          <tr>
            {columns.map((c) => (
              <th
                key={c.key}
                className={c.num ? "num" : undefined}
                style={c.width ? { width: c.width } : undefined}
              >
                {c.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => (
            <tr
              key={rowKey(row, i)}
              className={rowHighlight?.(row) ? "row--mine" : undefined}
            >
              {columns.map((c) => (
                <td key={c.key} className={c.num ? "num" : undefined}>
                  {c.render(row, i)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      {rows.length === 0 && (
        <p className="muted" style={{ padding: "0.6rem" }}>
          {emptyMessage}
        </p>
      )}
    </div>
  );
}

/* ------------------------------------------------------------------- Forms */

interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
  options: { value: string; label: string }[];
  /** Inline label to the left, like the original's Setup rows. Pass a string. */
  inlineLabel?: string | false;
}

/** Bevelled select. `inlineLabel` gives the original's label-left layout. */
export function Select({
  label,
  options,
  inlineLabel,
  className = "",
  ...rest
}: SelectProps) {
  const id = useId();
  const control = (
    <select id={rest.id ?? id} className={className} {...rest}>
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
  if (inlineLabel) {
    return (
      <div className="setuprow">
        <span className="setuprow__label">{inlineLabel}</span>
        <div className="setuprow__control">{control}</div>
      </div>
    );
  }
  return (
    <div className="field">
      {label && <label htmlFor={rest.id ?? id}>{label}</label>}
      {control}
    </div>
  );
}

interface TextFieldProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  inlineLabel?: string;
}

/** Bevelled text/number input. */
export function TextField({
  label,
  inlineLabel,
  className = "",
  ...rest
}: TextFieldProps) {
  const id = useId();
  const control = <input id={rest.id ?? id} className={className} {...rest} />;
  if (inlineLabel) {
    return (
      <div className="setuprow">
        <span className="setuprow__label">{inlineLabel}</span>
        <div className="setuprow__control">{control}</div>
      </div>
    );
  }
  return (
    <div className="field">
      {label && <label htmlFor={rest.id ?? id}>{label}</label>}
      {control}
    </div>
  );
}

interface CheckboxProps {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

/** The original's square checkbox with a light tick. */
export function Checkbox({ label, checked, onChange, disabled }: CheckboxProps) {
  return (
    <label className="radio">
      <input
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={(e) => onChange(e.target.checked)}
      />
      {label}
    </label>
  );
}

/* ---------------------------------------------------------------- Feedback */

interface StatProps {
  label: string;
  value: ReactNode;
  /** Gold value, used for headline numbers. */
  emphasis?: boolean;
}

/** Small stat tile. */
export function Stat({ label, value, emphasis }: StatProps) {
  return (
    <div className="stat">
      <div className="stat__label">{label}</div>
      <div className={`stat__value ${emphasis ? "stat__value--gold" : ""}`}>{value}</div>
    </div>
  );
}

interface MeterProps {
  label: string;
  /** 0-100. */
  value: number;
  suffix?: string;
}

/** Pitch-wear / spin / fitness meter bar. */
export function Meter({ label, value, suffix }: MeterProps) {
  return (
    <div className="meter">
      <span style={{ flex: "0 0 76px" }}>{label}</span>
      <span className="meter__track">
        <span className="meter__thumb" style={{ left: `${Math.max(0, Math.min(100, value))}%` }} />
      </span>
      {suffix && <span>{suffix}</span>}
    </div>
  );
}

interface BadgeProps {
  children: ReactNode;
  tone?: "neutral" | "ok" | "warn";
}

/** Rounded status pill. */
export function Badge({ children, tone = "neutral" }: BadgeProps) {
  const cls =
    tone === "ok" ? "pill pill--ok" : tone === "warn" ? "pill pill--warn" : "pill";
  return <span className={cls}>{children}</span>;
}

/** Confidence blocks, as the original uses for bowler/batsman confidence. */
export function ConfidenceBlocks({ value, max = 5 }: { value: number; max?: number }) {
  return (
    <span className="confblocks">
      {Array.from({ length: max }, (_, i) => (
        <span
          key={i}
          className={`confblocks__block ${i < value ? "confblocks__block--on" : ""}`}
        />
      ))}
    </span>
  );
}

/* ------------------------------------------------------------------ Layout */

interface GridProps {
  cols?: 2 | "side";
  children: ReactNode;
}

/** Two-column responsive grid used across the screens. */
export function Grid({ cols = 2, children }: GridProps) {
  return (
    <div className={`grid ${cols === "side" ? "grid--side" : "grid--2"}`}>{children}</div>
  );
}

/** Vertical stack. */
export function Stack({ children }: { children: ReactNode }) {
  return <div className="stack">{children}</div>;
}

/* ------------------------------------------------------------------- Icons */

/**
 * Inline SVG icons in the original's flat style. Kept here so every screen
 * uses the same set rather than ad-hoc glyphs.
 */
const ICON_PATHS: Record<string, string> = {
  news: "M2 5h12v8H2z M2 5l6 4 6-4",
  ball: "M7 1a6 6 0 100 12A6 6 0 007 1zm0 0v12M1 7h12",
  globe: "M7 1a6 6 0 100 12A6 6 0 007 1zM1 7h12M7 1c2 2 2 10 0 12M7 1C5 3 5 11 7 13",
  list: "M2 3h10v2H2zM2 7h10v2H2zM2 11h7v2H2z",
  search: "M6 1a5 5 0 100 10A5 5 0 006 1zm3 8l4 4",
  chart: "M2 12V2h2v10zM6 12V6h2v6zM10 12V4h2v8z",
  team: "M7 1a3 3 0 100 6 3 3 0 000-6zM2 13c0-3 2-5 5-5s5 2 5 5z",
  options: "M7 1l1.5 3.5L12 6l-3 2.5.8 3.8L7 10.5 4.2 12.3 5 8.5 2 6l3.5-1.5z",
  save: "M2 2h9l3 3v9H2zM4 4v7h7V6H7V4z",
};

interface IconProps {
  name: keyof typeof ICON_PATHS | string;
  size?: number;
  className?: string;
}

/** A single icon from the shared set. */
export function Icon({ name, size = 16, className = "" }: IconProps) {
  const path = ICON_PATHS[name] ?? ICON_PATHS.ball;
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 14 14"
      className={className}
      aria-hidden="true"
      focusable="false"
    >
      <path
        d={path}
        fill="none"
        stroke="currentColor"
        strokeWidth="1.3"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

/* --------------------------------------------------------------- Transitions */

/** Subtle fade used when switching tabs, so screens don't hard-cut. */
export function Fade({ children, keyName }: { children: ReactNode; keyName?: string }) {
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    const t = setTimeout(() => setVisible(true), 10);
    return () => clearTimeout(t);
  }, [keyName]);
  return (
    <div className={visible ? "fade fade--in" : "fade"} aria-hidden={!visible}>
      {children}
    </div>
  );
}
