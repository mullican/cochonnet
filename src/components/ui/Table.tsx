import type { HTMLAttributes, TdHTMLAttributes, ThHTMLAttributes } from 'react';

export function Table({
  className = '',
  ...props
}: HTMLAttributes<HTMLTableElement>) {
  return (
    // min-w-max is what makes a wide table scroll instead of squeezing its
    // columns until the text wraps - the difference between a readable
    // standings sheet on an iPad and an unreadable one.
    <div className="relative w-full overflow-x-auto overscroll-x-contain">
      <table
        className={`w-full min-w-max caption-bottom text-sm ${className}`}
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
      className={`h-12 whitespace-nowrap px-4 text-left align-middle font-medium text-gray-500 pointer-coarse:h-14 pointer-coarse:px-3 ${className}`}
      {...props}
    />
  );
}

export function TableCell({
  className = '',
  ...props
}: TdHTMLAttributes<HTMLTableCellElement>) {
  return (
    <td
      className={`whitespace-nowrap p-4 align-middle pointer-coarse:px-3 pointer-coarse:py-4 ${className}`}
      {...props}
    />
  );
}
