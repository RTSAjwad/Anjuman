// A generic shadcn-style data table over TanStack Table (v9), manual mode.
// `data` is the server's page; sort/filter/pagination remain server-owned. The
// caller supplies column defs + an `onRowClick`, and this renders the standard
// shadcn <Table> markup with <table.FlexRender />.
import { useTable, type ColumnDef, type RowData } from "@tanstack/react-table";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "~/components/ui/table";
import {
  browserTableFeatures,
  type BrowserTableFeatures,
} from "~/lib/data-table-features";

interface DataTableProps<TData extends RowData> {
  columns: ColumnDef<BrowserTableFeatures, TData>[];
  data: TData[];
  /** Return a string key + selected flag per row. */
  getRowKey: (row: TData) => string;
  isSelected: (row: TData) => boolean;
  onRowClick: (row: TData) => void;
}

export function BrowserDataTable<TData extends RowData>({
  columns,
  data,
  getRowKey,
  isSelected,
  onRowClick,
}: DataTableProps<TData>) {
  const table = useTable({
    features: browserTableFeatures,
    columns,
    data,
  });

  return (
    <Table>
      <TableHeader>
        {table.getHeaderGroups().map((headerGroup) => (
          <TableRow key={headerGroup.id}>
            {headerGroup.headers.map((header) => (
              <TableHead key={header.id}>
                {header.isPlaceholder ? null : <table.FlexRender header={header} />}
              </TableHead>
            ))}
          </TableRow>
        ))}
      </TableHeader>
      <TableBody>
        {table.getRowModel().rows.length ? (
          table.getRowModel().rows.map((row) => {
            const original = row.original as TData;
            return (
              <TableRow
                key={getRowKey(original)}
                data-state={isSelected(original) ? "selected" : undefined}
                onClick={() => onRowClick(original)}
                className="cursor-pointer"
              >
                {row.getVisibleCells().map((cell) => (
                  <TableCell key={cell.id}>
                    <table.FlexRender cell={cell} />
                  </TableCell>
                ))}
              </TableRow>
            );
          })
        ) : (
          <TableRow>
            <TableCell colSpan={columns.length} className="h-24 text-center">
              No results.
            </TableCell>
          </TableRow>
        )}
      </TableBody>
    </Table>
  );
}
