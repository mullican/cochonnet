import type { HTMLAttributes, TdHTMLAttributes, ThHTMLAttributes } from 'react';

export function Table({
  className = '',
  ...props
}: HTMLAttributes<HTMLTableElement>) {
  return (
    <div className="relative w-full overflow-auto">
      <table
        className={`w-full caption-bottom text-sm ${className}`}
        {...props}
      />
    </div>
  );
}

export function TableHeader({
  className = '',
  ...props
}: HTMLAttributes<HTMLTableSectionElement>) {
  return <thead className={`border-b bg-gray-50 ${className}`} {...props} />;
}

export function TableBody({
  className = '',
  ...props
}: HTMLAttributes<HTMLTableSectionElement>) {
  return (
    <tbody className={`[&_tr:last-child]:border-0 ${className}`} {...props} />
  );
}

export function TableFooter({
  className = '',
  ...props
}: HTMLAttributes<HTMLTableSectionElement>) {
  return (
    <tfoot
      className={`border-t bg-gray-50 font-medium ${className}`}
      {...props}
    />
  );
}

export function TableRow({
  className = '',
  ...props
}: HTMLAttributes<HTMLTableRowElement>) {
  return (
    <tr
      className={`border-b transition-colors hover:bg-gray-50 ${className}`}
      {...props}
    />
  );
}

export function TableHead({
  className = '',
  ...props
}: ThHTMLAttributes<HTMLTableCellElement>) {
  return (
    <th
      className={`h-12 px-4 text-left align-middle font-medium text-gray-500 ${className}`}
      {...props}
    />
  );
}

export function TableCell({
  className = '',
  ...props
}: TdHTMLAttributes<HTMLTableCellElement>) {
  return (
    <td className={`p-4 align-middle ${className}`} {...props} />
  );
}
