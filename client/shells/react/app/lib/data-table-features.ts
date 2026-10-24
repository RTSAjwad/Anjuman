// TanStack Table v9 feature set for the browser's data table. We run in *manual*
// mode: sort/filter/pagination are owned by the core (server-paginated), so we
// only opt into features that are local/render-only. The core row model is always
// included; column visibility is the one optional feature we enable now (cheap,
// and useful for future column toggling). Register no filter/sort/pagination
// features — those stay on the server.
import {
  columnVisibilityFeature,
  createCoreRowModel,
  tableFeatures,
} from "@tanstack/react-table";

export const browserTableFeatures = tableFeatures({
  columnVisibilityFeature,
  coreRowModel: createCoreRowModel(),
});

export type BrowserTableFeatures = typeof browserTableFeatures;
