export interface Column {
  id: string;
  label: string;
  align?: 'start' | 'end';
  /** Ширина колонки в grid-template: `1fr`, `120px`. */
  width?: string;
  sortable?: boolean;
}
