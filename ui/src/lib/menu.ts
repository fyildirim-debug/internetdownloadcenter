// Menü ve bağlam menüsü öğe tipi.
export interface MenuItem {
  label?: string;
  shortcut?: string;
  icon?: string;
  action?: () => void;
  disabled?: boolean;
  checked?: boolean;
  radio?: boolean;
  separator?: boolean;
  bold?: boolean;
  submenu?: MenuItem[];
}

export const SEP: MenuItem = { separator: true };

export interface ToolButton {
  id: string;
  label: string;
  icon: string;
  action: () => void;
  disabled?: boolean;
  sepBefore?: boolean;
}

/** Ana penceredeki ağaç süzgeci. */
export type Filter = { kind: 'all' | 'category' | 'unfinished' | 'finished' | 'queue'; id?: string };
