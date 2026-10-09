//! App chrome: top bar (clock / title / flags), left sidebar, bottom action bar.
//! Mirrors the Cricket Coach 2014 desktop layout.

import type { ReactNode } from "react";

export interface SidebarItem {
  id: string;
  label: string;
  icon: "news" | "team" | "intl" | "dom" | "search" | "rank" | "records" | "jobs" | "options" | "save";
  badge?: number;
  active?: boolean;
  onClick?: () => void;
}

interface Props {
  clock: string;
  clockTitle: string;
  title: string;
  subtitle?: string;
  homeFlag?: string;
  awayFlag?: string;
  items: SidebarItem[];
  quickSearch?: ReactNode;
  actionBar: ReactNode;
  children: ReactNode;
}

export default function AppShell({
  clock,
  clockTitle,
  title,
  subtitle,
  homeFlag,
  awayFlag,
  items,
  quickSearch,
  actionBar,
  children,
}: Props) {
  return (
    <div className="app">
      <header className="topbar">
        <div className="topbar__clock">
          <b>{clock}</b>
          <span>{clockTitle}</span>
        </div>
        <div className="topbar__title">
          <h1>{title}</h1>
          {subtitle && <p>{subtitle}</p>}
        </div>
        <div className="topbar__flags">
          {homeFlag && <span className="flag flag--home">{homeFlag}</span>}
          {awayFlag && <span className="flag flag--away">{awayFlag}</span>}
        </div>
      </header>

      <nav className="sidebar" aria-label="Main navigation">
        <div className="sidebar__nav">
          {items.map((item) => (
            <button
              key={item.id}
              type="button"
              className={`navrow navrow--${item.icon} ${
                item.active ? "navrow--active" : ""
              }`}
              onClick={item.onClick}
              aria-current={item.active ? "page" : undefined}
            >
              <span className="navrow__icon" aria-hidden="true">
                {iconGlyph(item.icon)}
              </span>
              <span className="navrow__label">{item.label}</span>
              {item.badge ? (
                <span className="navrow__badge">{item.badge}</span>
              ) : null}
            </button>
          ))}
        </div>

        {quickSearch && <div className="sidebar__quicksearch">{quickSearch}</div>}
        <div className="sidebar__spacer" />
        <div className="sidebar__section" />
      </nav>

      <div className="content">
        <div className="content__inner">{children}</div>
      </div>

      <div className="actionbar">{actionBar}</div>
    </div>
  );
}

function iconGlyph(icon: SidebarItem["icon"]): string {
  switch (icon) {
    case "news":
      return "✉";
    case "team":
      return "●";
    case "intl":
      return "🌐";
    case "dom":
      return "▤";
    case "search":
      return "⌕";
    case "rank":
      return "●";
    case "records":
      return "●";
    case "jobs":
      return "✎";
    case "options":
      return "✓";
    case "save":
      return "▣";
    default:
      return "●";
  }
}
