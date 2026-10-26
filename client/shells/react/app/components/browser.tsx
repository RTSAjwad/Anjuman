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
  eventBrowserMoveCard,
  eventBrowserSuspendNote,
  eventBrowserUnsuspendNote,
  eventBrowserBuryNote,
  eventBrowserUnburyNote,
  eventNoteSave,
  eventNoteAddTags,
  eventNoteRemoveTags,
  eventNoteDelete,
} from "shared_types/app";
import { cn } from "~/lib/utils";
import { Input } from "~/components/ui/input";
import { Button } from "~/components/ui/button";
import { Toggle } from "~/components/ui/toggle";
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
import { ChipInput } from "~/components/ui/chip-input";
import { Separator } from "~/components/ui/separator";
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

  // Fetch the current view's data. Re-fires whenever the route changes (Cards ↔
  // Notes) so the newly-selected list is populated; note-types are fetched once.
  useEffect(() => {
    if (!initialized) {
      setInitialized(true);
      update(eventNoteTypesRequested());
    }
    apply(1);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [route]);

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
        <NoteDetailsPanel
          route={route}
          cardSelected={props.cardSelected}
          noteSelected={props.noteSelected}
          noteTypes={props.noteTypes}
          canManageNotes={props.canManageNotes}
          update={update}
        />
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
  { accessorKey: "sort_field", header: "Sort field", cell: ({ getValue }) => (getValue() as string | null) ?? "" },
  { accessorKey: "template_name", header: "Card type" },
  { accessorKey: "due_at", header: "Due date", cell: ({ getValue }) => (getValue() as string | null) ?? "" },
  { accessorKey: "state", header: "State" },
  {
    id: "flag",
    header: "Flag",
    cell: ({ row }) => <FlagSwatch flag={row.original.flag} />,
  },
  { accessorKey: "deck_path", header: "Deck" },
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

// Flatten the deck tree into a flat list of { id, path } with the full
// `::`-joined ancestor path (matching the browser's Deck column), for the
// change-deck picker.
function flatDeckOptions(decks: DeckSummary[]): { id: bigint; path: string }[] {
  const out: { id: bigint; path: string }[] = [];
  function walk(ds: DeckSummary[], path: string[]) {
    for (const d of ds) {
      const next = [...path, d.title];
      out.push({ id: d.id, path: next.join("::") });
      walk(d.children, next);
    }
  }
  walk(decks, []);
  return out;
}

export function CardActions({ card, decks, canManageNotes, update }: { card: CardBrowserView | null; decks: DeckSummary[]; canManageNotes: boolean; update: (e: Event) => void }) {
  const [rescheduleOpen, setRescheduleOpen] = useState(false);
  const [days, setDays] = useState("");
  const [moveOpen, setMoveOpen] = useState(false);
  const [targetDeck, setTargetDeck] = useState("");

  if (!card) {
    return null;
  }

  function confirmReschedule() {
    const d = Number(days.trim());
    if (Number.isFinite(d)) {
      update(eventBrowserRescheduleCard(card!.card_id, BigInt(d)));
      setDays("");
      setRescheduleOpen(false);
    }
  }

  function confirmMove() {
    if (targetDeck !== "") {
      update(eventBrowserMoveCard(card!.card_id, BigInt(targetDeck)));
      setTargetDeck("");
      setMoveOpen(false);
    }
  }

  return (
    <div className="flex items-center gap-2">
      <Separator orientation="vertical" />
      <span className="text-sm font-medium text-muted-foreground">Card Actions</span>
      <ButtonGroup>
        <Toggle
          variant="outline"
          pressed={card.suspended}
          onPressedChange={(v) =>
            update(v ? eventBrowserSuspendCard(card.card_id) : eventBrowserUnsuspendCard(card.card_id))
          }
        >
          Suspend Card
        </Toggle>
        <Toggle
          variant="outline"
          pressed={card.buried}
          onPressedChange={(v) =>
            update(v ? eventBrowserBuryCard(card.card_id) : eventBrowserUnburyCard(card.card_id))
          }
        >
          Bury Card
        </Toggle>
        <Button variant="outline" onClick={() => setRescheduleOpen(true)}>Reschedule</Button>
        <Button variant="outline" onClick={() => setMoveOpen(true)}>Change deck</Button>
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
      </ButtonGroup>

      <Separator orientation="vertical" />
      <span className="text-sm font-medium text-muted-foreground">Note Actions</span>
      <NoteActions
        noteId={card.note_id}
        noteSuspended={card.note_suspended}
        noteBuried={card.note_buried}
        canManageNotes={canManageNotes}
        update={update}
      />

      <Dialog open={moveOpen} onOpenChange={setMoveOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Change deck</DialogTitle>
            <DialogDescription>Move this card to a different deck.</DialogDescription>
          </DialogHeader>
          <Select
            items={flatDeckOptions(decks).map((d) => ({ value: String(d.id), label: d.path }))}
            value={targetDeck}
            onValueChange={(v) => { if (v !== null) setTargetDeck(v); }}
          >
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                {flatDeckOptions(decks).map((d) => (
                  <SelectItem key={String(d.id)} value={String(d.id)}>
                    {d.path}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
          <DialogFooter>
            <Button variant="outline" onClick={() => setMoveOpen(false)}>Cancel</Button>
            <Button onClick={confirmMove} disabled={targetDeck === ""}>Move</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

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

export function NoteActions({
  noteId,
  noteSuspended,
  noteBuried,
  canManageNotes,
  update,
}: {
  noteId: bigint;
  noteSuspended: boolean | null;
  noteBuried: boolean | null;
  canManageNotes: boolean;
  update: (e: Event) => void;
}) {
  const [deleteOpen, setDeleteOpen] = useState(false);

  function confirmDelete() {
    update(eventNoteDelete(noteId));
    setDeleteOpen(false);
  }

  return (
    <>
      <ButtonGroup>
        {noteSuspended === null ? (
          <Button variant="outline" disabled={!canManageNotes} onClick={() => update(eventBrowserSuspendNote(noteId))}>
            Suspend Note
          </Button>
        ) : (
          <Toggle
            variant="outline"
            pressed={noteSuspended}
            onPressedChange={(v) =>
              update(v ? eventBrowserSuspendNote(noteId) : eventBrowserUnsuspendNote(noteId))
            }
          >
            Suspend Note
          </Toggle>
        )}
        {noteBuried === null ? (
          <Button variant="outline" disabled={!canManageNotes} onClick={() => update(eventBrowserBuryNote(noteId))}>
            Bury Note
          </Button>
        ) : (
          <Toggle
            variant="outline"
            pressed={noteBuried}
            onPressedChange={(v) =>
              update(v ? eventBrowserBuryNote(noteId) : eventBrowserUnburyNote(noteId))
            }
          >
            Bury Note
          </Toggle>
        )}
        <Button variant="destructive" onClick={() => setDeleteOpen(true)} disabled={!canManageNotes}>Delete Note</Button>
      </ButtonGroup>

      <Dialog open={deleteOpen} onOpenChange={setDeleteOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete note</DialogTitle>
            <DialogDescription>
              This deletes the note and all of its cards. This cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteOpen(false)}>Cancel</Button>
            <Button variant="destructive" onClick={confirmDelete}>Delete</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

function NoteDetailsPanel({
  route,
  cardSelected,
  noteSelected,
  noteTypes,
  canManageNotes,
  update,
}: {
  route: "Cards" | "Notes";
  cardSelected: CardBrowserView | null;
  noteSelected: NoteView | null;
  noteTypes: NoteTypeView[];
  canManageNotes: boolean;
  update: (e: Event) => void;
}) {
  // Derive the note under selection for the active mode (cards select their
  // note, notes select it directly). Both are core-owned and re-synced, so this
  // stays read-only; the panel keeps *only* the in-progress field edits locally.
  // Tags are note-level: notes mode carries them on the NoteView; cards mode has
  // no tags yet (the pending core carve-out adds CardBrowserView.tags).
  const note =
    route === "Cards"
      ? cardSelected
        ? {
            id: cardSelected.note_id,
            note_type_id: cardSelected.note_type_id,
            note_type_name: cardSelected.note_type_name,
            fields: cardSelected.fields,
            tags: [...cardSelected.tags],
          }
        : null
      : noteSelected
        ? {
            id: noteSelected.id,
            note_type_id: noteSelected.note_type_id,
            note_type_name: noteSelected.note_type_name,
            fields: noteSelected.fields,
            tags: [...noteSelected.tags],
          }
        : null;

  const fieldNames = (() => {
    if (!note) return [];
    const nt = noteTypes.find((t) => t.id === note.note_type_id);
    if (nt && nt.field_names.length > 0) return [...nt.field_names];
    // Fallback: iterate the note's own fields (order not guaranteed).
    return Array.from(note.fields.keys());
  })();

  // In-progress edits + the last-saved snapshot, both re-seeded whenever the
  // selected note changes. `committed` is the source of truth for dirty-ness
  // rather than the (possibly not-yet-refreshed) `note.fields`, so the Save
  // button disables immediately after a save in both cards and notes modes.
  const [edits, setEdits] = useState<Map<string, string>>(new Map());
  const [committed, setCommitted] = useState<Map<string, string>>(new Map());
  useEffect(() => {
    const seed = new Map(note ? note.fields : []);
    setEdits(seed);
    setCommitted(seed);
  }, [note?.id]);

  if (!note) {
    return <aside className="w-72 shrink-0 border-l" />;
  }

  const dirty = (() => {
    if (edits.size !== committed.size) return true;
    for (const [k, v] of committed) {
      if (edits.get(k) !== v) return true;
    }
    return false;
  })();

  function setField(name: string, value: string) {
    setEdits((prev) => {
      const next = new Map(prev);
      next.set(name, value);
      return next;
    });
  }

  function save() {
    const snapshot = new Map(edits);
    update(eventNoteSave(note!.id, null, snapshot));
    setCommitted(snapshot);
  }

  // Tag add/remove are immediate (not part of the field Save) and map to the
  // core's NoteAddTags/NoteRemoveTags; the core re-syncs the selected note/card
  // tags after each, so the chip input updates reactively.
  function addTag(tag: string) {
    update(eventNoteAddTags(note!.id, [tag]));
  }
  function removeTag(tag: string) {
    update(eventNoteRemoveTags(note!.id, [tag]));
  }

  return (
    <aside className="flex w-72 shrink-0 flex-col gap-3 border-l p-3">
      <h3 className="text-sm font-semibold">Note</h3>
      <p className="text-xs text-muted-foreground">{note.note_type_name}</p>
      <div className="flex flex-col gap-2">
        {fieldNames.map((name) => (
          <div key={name} className="flex flex-col gap-1">
            <Label className="text-xs text-muted-foreground">{name}</Label>
            <Input
              value={edits.get(name) ?? ""}
              onChange={(e) => setField(name, e.target.value)}
              disabled={!canManageNotes}
            />
          </div>
        ))}
      </div>
      <div className="flex flex-col gap-1">
        <Label className="text-xs text-muted-foreground">Tags</Label>
        <ChipInput
          tags={note.tags}
          onAdd={addTag}
          onRemove={removeTag}
          disabled={!canManageNotes}
        />
      </div>
      <Button onClick={save} disabled={!dirty || !canManageNotes}>Save</Button>
    </aside>
  );
}
