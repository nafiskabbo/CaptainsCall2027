import type { ReactNode } from "react";
import { Icon } from "../ui";

export interface SidebarItem {
  id: string;
  label: string;
  icon: string;
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

/**
 * App chrome: blue-gradient top bar (clock / title / flags), the original's
 * icon sidebar, the green field backdrop and the bottom action bar.
 */
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
              className={`navrow ${item.active ? "navrow--active" : ""}`}
              onClick={item.onClick}
              aria-current={item.active ? "page" : undefined}
            >
              <span className="navrow__icon" aria-hidden="true">
                <Icon name={item.icon} size={15} />
              </span>
              <span className="navrow__label">{item.label}</span>
              {item.badge ? <span className="navrow__badge">{item.badge}</span> : null}
            </button>
          ))}
        </div>

        {quickSearch && <div className="sidebar__quicksearch">{quickSearch}</div>}
        <div className="sidebar__spacer" />
      </nav>

      <div className="content">
        <div className="content__inner">{children}</div>
      </div>

      <div className="actionbar">{actionBar}</div>
    </div>
  );
}
