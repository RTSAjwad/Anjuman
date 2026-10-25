import { useEffect, useState } from "react";
import type { ColumnDef } from "@tanstack/react-table";
import type { BrowserTableFeatures } from "~/lib/data-table-features";
import { BrowserDataTable } from "./data-table";
import type {
  CardBrowserFilter,
  CardBrowserView,
  DeckSummary,
  NoteBrowserFilter,
  NoteView,
  NoteTypeView,
} from "shared_types/app";
import {
  CardBrowserFilter as CardBrowserFilterCtor,
  NoteBrowserFilter as NoteBrowserFilterCtor,
} from "shared_types/app";
import type { Event } from "shared_types/app";
import {
  eventCardsRequested,
  eventNotesRequested,
  eventNoteTypesRequested,
  eventBrowserSelectCard,
  eventNoteBrowserSelectNote,
  eventBrowserSuspendCard,
  eventBrowserUnsuspendCard,
  eventBrowserBuryCard,
  eventBrowserUnburyCard,
  eventBrowserRescheduleCard,
  eventBrowserFlagCard,
} from "shared_types/app";
import { cn } from "~/lib/utils";
import { Input } from "~/components/ui/input";
import { Button } from "~/components/ui/button";
import { Badge } from "~/components/ui/badge";
import { Switch } from "~/components/ui/switch";
import { Label } from "~/components/ui/label";

import {
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from "~/components/ui/pagination";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "~/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "~/components/ui/collapsible";
import { ButtonGroup } from "~/components/ui/button-group";
import { ChevronRight, Flag, Search } from "lucide-react";

const CARD_SORT_OPTIONS = [
  ["created_at", "Created"],
  ["due_at", "Due"],
  ["deck", "Deck"],
  ["question", "Question"],
] as const;
const NOTE_SORT_OPTIONS = [
  ["created_at", "Created"],
  ["note_type", "Note type"],
  ["deck", "Deck"],
  ["question", "Question"],
] as const;
const STATE_OPTIONS = [
  ["new", "New"],
  ["learning", "Learning"],
  ["review", "Review"],
  ["relearning", "Relearning"],
] as const;
const FLAG_OPTIONS = [
  { value: "0", label: "No flag" },
  { value: "1", label: "Red", color: "text-red-500" },
  { value: "2", label: "Orange", color: "text-orange-500" },
  { value: "3", label: "Green", color: "text-green-500" },
  { value: "4", label: "Blue", color: "text-blue-500" },
  { value: "5", label: "Pink", color: "text-pink-500" },
  { value: "6", label: "Turquoise", color: "text-teal-500" },
  { value: "7", label: "Purple", color: "text-purple-500" },
];
function flagColor(value: string): string | undefined {
  return FLAG_OPTIONS.find((o) => o.value === value)?.color;
}

interface FilterState {
  deck: string;
  noteType: string;
  state: string;
  flag: string;
  sort: string;
  query: string;
}

type BrowserScreenProps = {
  route: "Cards" | "Notes";
  decks: DeckSummary[];
  cards: CardBrowserView[];
  cardPage: bigint;
  cardPerPage: bigint;
  cardTotal: bigint;
  cardSelected: CardBrowserView | null;
  cardError: string | null;
  notes: NoteView[];
  notePage: bigint;
  notePerPage: bigint;
  noteTotal: bigint;
  noteSelected: NoteView | null;
  noteError: string | null;
  noteTypes: NoteTypeView[];
  canManageNotes: boolean;
  update: (event: Event) => void;
};

export function BrowserScreen(props: BrowserScreenProps) {
  const { route, update } = props;

  // Hoisted filter state — shared by the sidebar and pagination (mirrors the
  // Leptos `BrowserState`). A change always refetches page 1.
  const [filter, setFilter] = useState<FilterState>({
    deck: "",
    noteType: "",
    state: "",
    flag: "",
    sort: "created_at",
    query: "",
  });
  const [initialized, setInitialized] = useState(false);

  function buildCardFilter(page: number): CardBrowserFilter {
    return new CardBrowserFilterCtor(
      filter.deck === "" || filter.deck === "0" ? [] : [BigInt(filter.deck)],
      filter.noteType === "" || filter.noteType === "0" ? [] : [BigInt(filter.noteType)],
      filter.query.trim() === "" ? null : filter.query.trim(),
      filter.state === "" ? [] : [filter.state],
      filter.flag === "" ? [] : [BigInt(filter.flag)],
      filter.sort,
      BigInt(page),
      BigInt(50),
    );
  }
  function buildNoteFilter(page: number): NoteBrowserFilter {
    return new NoteBrowserFilterCtor(
      filter.deck === "" || filter.deck === "0" ? [] : [BigInt(filter.deck)],
      filter.noteType === "" || filter.noteType === "0" ? [] : [BigInt(filter.noteType)],
      filter.query.trim() === "" ? null : filter.query.trim(),
      filter.sort,
      BigInt(page),
      BigInt(50),
    );
  }
  function apply(page: number) {
    if (route === "Cards") update(eventCardsRequested(buildCardFilter(page)));
    else update(eventNotesRequested(buildNoteFilter(page)));
  }

  // First entry: note-types + default fetch, once.
  useEffect(() => {
    if (initialized) return;
    setInitialized(true);
    update(eventNoteTypesRequested());
    apply(1);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialized, route]);

  function set<K extends keyof FilterState>(key: K, value: FilterState[K]) {
    setFilter((f) => ({ ...f, [key]: value }));
  }
  function toggle<K extends keyof FilterState>(key: K, value: string, resetValue: string) {
    setFilter((f) => {
      const next = { ...f, [key]: f[key] === value ? resetValue : value };
      return next;
    });
    apply(1);
  }
  function toggleSort(value: string) {
    setFilter((f) => ({ ...f, sort: f.sort === value ? "created_at" : value }));
    apply(1);
  }

  const page = route === "Cards" ? Number(props.cardPage) : Number(props.notePage);
  const perPage = route === "Cards" ? Math.max(1, Number(props.cardPerPage)) : Math.max(1, Number(props.notePerPage));
  const total = route === "Cards" ? Number(props.cardTotal) : Number(props.noteTotal);
  const pageCount = Math.max(1, Math.ceil(total / perPage));
  const listLabel = route === "Cards" ? "cards" : "notes";
  const error = route === "Cards" ? props.cardError : props.noteError;

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      {route === "Cards" ? (
        <CardActions card={props.cardSelected} update={update} />
      ) : null}
      <div className="flex min-h-0 flex-1 overflow-hidden">
      <aside className="w-56 shrink-0 overflow-y-auto border-r p-3">
        <form
          onSubmit={(e) => {
            e.preventDefault();
            apply(1);
          }}
          className="w-full"
        >
          <ButtonGroup className="w-full">
            <Input
              value={filter.query}
              onChange={(e) => set("query", e.target.value)}
              placeholder="Search…"
            />
            <Button type="submit" variant="outline" size="icon" aria-label="Search">
              <Search />
            </Button>
          </ButtonGroup>
        </form>

        <FilterBranch title="Decks">
          <DeckFilterTree
            decks={props.decks}
            selected={filter.deck}
            onSelect={(id) => toggle("deck", id, "")}
          />
        </FilterBranch>

        <FilterBranch title="Note types">
          {props.noteTypes.map((nt) => {
            const id = String(nt.id);
            return (
              <button
                key={id}
                onClick={() => toggle("noteType", id, "")}
                className={cn(
                  "flex w-full items-center rounded-sm px-3 py-1.5 text-left text-sm",
                  filter.noteType === id && "bg-accent text-accent-foreground",
                )}
              >
                {nt.name}
              </button>
            );
          })}
        </FilterBranch>

        {route === "Cards" ? (
          <>
            <FilterBranch title="State">
              {STATE_OPTIONS.map(([v, label]) => (
                <button
                  key={v}
                  onClick={() => toggle("state", v, "")}
                  className={cn(
                    "flex w-full items-center rounded-sm px-3 py-1.5 text-left text-sm",
                    filter.state === v && "bg-accent text-accent-foreground",
                  )}
                >
                  {label}
                </button>
              ))}
            </FilterBranch>

            <FilterBranch title="Flag">
              {FLAG_OPTIONS.slice(1).map((opt) => (
                <button
                  key={opt.value}
                  onClick={() => toggle("flag", opt.value, "")}
                  className={cn(
                    "flex w-full items-center rounded-sm px-3 py-1.5 text-left text-sm",
                    filter.flag === opt.value && "bg-accent text-accent-foreground",
                  )}
                >
                  <Flag className={cn("mr-2 size-4", opt.color, "fill-current")} />
                  {opt.label}
                </button>
              ))}
            </FilterBranch>
          </>
        ) : null}

        <FilterBranch title="Sort">
          {(route === "Cards" ? CARD_SORT_OPTIONS : NOTE_SORT_OPTIONS).map(([v, label]) => (
            <button
              key={v}
              onClick={() => toggleSort(v)}
              className={cn(
                "flex w-full items-center rounded-sm px-3 py-1.5 text-left text-sm",
                filter.sort === v && "bg-accent text-accent-foreground",
              )}
            >
              {label}
            </button>
          ))}
        </FilterBranch>

        {error ? <p className="mt-2 text-xs text-destructive">Could not load {listLabel}: {error}</p> : null}
      </aside>

      <div className="flex min-h-0 flex-1">
        <div className="flex min-h-0 flex-1 flex-col">
          <div className="min-h-0 flex-1 overflow-auto">
            {route === "Cards" ? <CardsTable {...props} /> : <NotesTable {...props} />}
          </div>
          <div className="flex items-center justify-between border-t px-4 py-2">
            <span className="text-sm text-muted-foreground">{total} {listLabel}</span>
            <Pagination className="w-auto">
              <PaginationContent>
                <PaginationItem>
                  <PaginationPrevious onClick={() => apply(Math.max(1, page - 1))} />
                </PaginationItem>
                <PaginationItem>
                  <PaginationLink isActive>{page}</PaginationLink>
                </PaginationItem>
                <PaginationItem>
                  <PaginationNext onClick={() => apply(Math.min(pageCount, page + 1))} />
                </PaginationItem>
              </PaginationContent>
            </Pagination>
          </div>
        </div>
        {route === "Cards" ? <CardInspector {...props} /> : <NoteInspector {...props} />}
      </div>
      </div>
    </div>
  );
}

function FilterBranch({ title, children }: { title: string; children: React.ReactNode }) {
  const [open, setOpen] = useState(true);
  return (
    <Collapsible open={open} onOpenChange={setOpen} className="mt-3">
      <CollapsibleTrigger className="flex w-full items-center gap-1 px-1 py-1 text-sm font-medium">
        <ChevronRight className={cn("size-3.5 transition-transform", open && "rotate-90")} />
        {title}
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="flex flex-col">{children}</div>
      </CollapsibleContent>
    </Collapsible>
  );
}

function DeckFilterTree({ decks, selected, onSelect }: { decks: DeckSummary[]; selected: string; onSelect: (id: string) => void }) {
  return (
    <ul className="flex flex-col">
      {decks.map((d) => (
        <DeckFilterNode key={String(d.id)} deck={d} selected={selected} onSelect={onSelect} depth={0} />
      ))}
    </ul>
  );
}

function DeckFilterNode({ deck, selected, onSelect, depth }: { deck: DeckSummary; selected: string; onSelect: (id: string) => void; depth: number }) {
  const [open, setOpen] = useState(false);
  const hasChildren = deck.children.length > 0;
  const id = String(deck.id);
  const isSel = selected === id;

  return (
    <li>
      <div className="flex items-center" style={{ paddingInlineStart: `${depth * 0.75}rem` }}>
        {hasChildren ? (
          <button onClick={(e) => { e.stopPropagation(); setOpen((v) => !v); }} className="flex size-5 shrink-0 items-center justify-center">
            <ChevronRight className={cn("size-3.5 transition-transform", open && "rotate-90")} />
          </button>
        ) : (
          <span className="size-5 shrink-0" />
        )}
        <button
          onClick={() => onSelect(id)}
          className={cn("flex flex-1 items-center rounded-sm px-2 py-1.5 text-left text-sm", isSel && "bg-accent text-accent-foreground")}
        >
          {deck.title}
        </button>
      </div>
      {hasChildren && open ? (
        <ul className="flex flex-col">
          {deck.children.map((c) => (
            <DeckFilterNode key={String(c.id)} deck={c} selected={selected} onSelect={onSelect} depth={depth + 1} />
          ))}
        </ul>
      ) : null}
    </li>
  );
}

const cardColumns: ColumnDef<BrowserTableFeatures, CardBrowserView>[] = [
  {
    accessorKey: "front",
    header: "Card",
    cell: ({ getValue, row }) => (
      <span>
        {getValue() as string}
        {row.original.suspended ? " (suspended)" : ""}
        {row.original.buried ? " (buried)" : ""}
      </span>
    ),
  },
  { accessorKey: "deck_title", header: "Deck" },
  { accessorKey: "state", header: "State" },
  {
    id: "flag",
    header: "Flag",
    cell: ({ row }) => <FlagSwatch flag={row.original.flag} />,
  },
];

const noteColumns: ColumnDef<BrowserTableFeatures, NoteView>[] = [
  { accessorKey: "sort_field", header: "Sort field", cell: ({ getValue }) => (getValue() as string | null) ?? "" },
  { accessorKey: "note_type_name", header: "Note type" },
  { accessorKey: "tags", header: "Tags", cell: ({ getValue }) => (getValue() as string[]).join(", ") },
];

function CardsTable({ cards, cardSelected, update }: Pick<BrowserScreenProps, "cards" | "cardSelected" | "update">) {
  return (
    <BrowserDataTable
      columns={cardColumns}
      data={cards}
      getRowKey={(c) => String(c.card_id)}
      isSelected={(c) => cardSelected?.card_id === c.card_id}
      onRowClick={(c) => update(eventBrowserSelectCard(c.card_id))}
    />
  );
}

function NotesTable({ notes, noteSelected, update }: Pick<BrowserScreenProps, "notes" | "noteSelected" | "update">) {
  return (
    <BrowserDataTable
      columns={noteColumns}
      data={notes}
      getRowKey={(n) => String(n.id)}
      isSelected={(n) => noteSelected?.id === n.id}
      onRowClick={(n) => update(eventNoteBrowserSelectNote(n.id))}
    />
  );
}

function FlagSwatch({ flag }: { flag: bigint }) {
  const n = Number(flag);
  const opt = FLAG_OPTIONS[n];
  if (!opt) return null;
  return (
    <span className="inline-flex items-center gap-1.5">
      <Flag className={cn("size-4", opt.color, n !== 0 && "fill-current")} />
      <span className="text-xs text-muted-foreground">{opt.label}</span>
    </span>
  );
}

function CardActions({ card, update }: { card: CardBrowserView | null; update: (e: Event) => void }) {
  const [rescheduleOpen, setRescheduleOpen] = useState(false);
  const [days, setDays] = useState("");

  if (!card) {
    return <div className="flex h-11 shrink-0 items-center border-b px-3 text-sm text-muted-foreground" />;
  }

  function confirmReschedule() {
    const d = Number(days.trim());
    if (Number.isFinite(d)) {
      update(eventBrowserRescheduleCard(card!.card_id, BigInt(d)));
      setDays("");
      setRescheduleOpen(false);
    }
  }

  return (
    <div className="flex h-11 shrink-0 items-center gap-4 border-b px-3">
      <div className="flex items-center gap-2">
        <Label className="text-sm text-muted-foreground">Suspended</Label>
        <Switch
          checked={card.suspended}
          onCheckedChange={(v) =>
            update(v ? eventBrowserSuspendCard(card.card_id) : eventBrowserUnsuspendCard(card.card_id))
          }
        />
      </div>
      <div className="flex items-center gap-2">
        <Label className="text-sm text-muted-foreground">Buried</Label>
        <Switch
          checked={card.buried}
          onCheckedChange={(v) =>
            update(v ? eventBrowserBuryCard(card.card_id) : eventBrowserUnburyCard(card.card_id))
          }
        />
      </div>
      <div className="flex items-center gap-2">
        <Label className="text-sm text-muted-foreground">Flag</Label>
        <Select
          items={FLAG_OPTIONS}
          value={String(card.flag)}
          onValueChange={(v) => { if (v !== null) update(eventBrowserFlagCard(card.card_id, Number(v))); }}
        >
          <SelectTrigger className="w-28">
            <Flag className={cn("size-4", String(card.flag) !== "0" && flagColor(String(card.flag)), String(card.flag) !== "0" && "fill-current")} />
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              {FLAG_OPTIONS.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  <Flag className={cn("size-4", opt.color, opt.value !== "0" && "fill-current")} />
                  {opt.label}
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
      </div>
      <Button variant="outline" onClick={() => setRescheduleOpen(true)}>Reschedule</Button>

      <Dialog open={rescheduleOpen} onOpenChange={setRescheduleOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Reschedule</DialogTitle>
            <DialogDescription>How many days from now?</DialogDescription>
          </DialogHeader>
          <Input type="number" value={days} onChange={(e) => setDays(e.target.value)} placeholder="days" />
          <DialogFooter>
            <Button variant="outline" onClick={() => setRescheduleOpen(false)}>Cancel</Button>
            <Button onClick={confirmReschedule}>Reschedule</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function CardInspector({ cardSelected }: Pick<BrowserScreenProps, "cardSelected">) {
  if (!cardSelected) {
    return <aside className="w-64 shrink-0 border-l" />;
  }

  return (
    <aside className="flex w-64 shrink-0 flex-col gap-3 border-l p-3">
      <h3 className="text-sm font-semibold">Card</h3>
      <p className="text-sm">{cardSelected.front}</p>
    </aside>
  );
}

function NoteInspector({ noteSelected }: Pick<BrowserScreenProps, "noteSelected">) {
  if (!noteSelected) {
    return <aside className="w-64 shrink-0 border-l" />;
  }
  return (
    <aside className="flex w-64 shrink-0 flex-col gap-2 border-l p-3">
      <h3 className="text-sm font-semibold">Note</h3>
      <p className="text-xs text-muted-foreground">{noteSelected.note_type_name}</p>
      <div className="flex flex-col gap-1">
        {Array.from(noteSelected.fields.entries()).map(([k, v]) => (
          <div key={k} className="text-sm">
            <span className="text-muted-foreground">{k}: </span>
            {v}
          </div>
        ))}
      </div>
      {noteSelected.tags.length > 0 ? (
        <div className="flex flex-wrap gap-1">
          {noteSelected.tags.map((t) => (
            <Badge key={t} variant="outline">{t}</Badge>
          ))}
        </div>
      ) : null}
    </aside>
  );
}
