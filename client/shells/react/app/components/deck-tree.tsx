import { useState } from "react";
import type { MouseEvent } from "react";
import type { DeckSummary } from "shared_types/app";
import { eventOpenDeck, eventOpenDeckOptions } from "shared_types/app";
import { cn } from "~/lib/utils";
import { Card, CardContent } from "~/components/ui/card";
import { Badge } from "~/components/ui/badge";
import { Button } from "~/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "~/components/ui/collapsible";
import { ChevronRight, Settings } from "lucide-react";

// The fixed grid for header + every row. The title column flexes; the three
// count columns and the action column are fixed so a parent's badges stay in
// column with its children's regardless of nesting depth (indentation lives in
// the title cell only).
const ROW_GRID = "[grid-template-columns:minmax(0,1fr)_4rem_4rem_4rem_2.5rem]";

type DeckTreeProps = {
  decks: DeckSummary[];
  error?: string | null;
  onOpenDeck: (deckId: bigint) => void;
  onOpenOptions: (deckId: bigint) => void;
};

export function DeckTree({ decks, error, onOpenDeck, onOpenOptions }: DeckTreeProps) {
  if (error) {
    return (
      <Card>
        <CardContent className="text-destructive">{error}</CardContent>
      </Card>
    );
  }

  if (decks.length === 0) {
    return (
      <Card>
        <CardContent className="text-muted-foreground">
          No decks yet. Create one to get started.
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="p-0">
      <div
        role="row"
        className={cn(
          "grid items-center gap-2 border-b px-3 py-2 text-xs font-medium text-muted-foreground",
          ROW_GRID,
        )}
      >
        <span>Deck</span>
        <span className="text-center">New</span>
        <span className="text-center">Learn</span>
        <span className="text-center">Due</span>
        <span aria-hidden="true" />
      </div>

      <div role="tree" className="flex flex-col">
        {decks.map((deck) => (
          <DeckRow
            key={String(deck.id)}
            deck={deck}
            depth={0}
            onOpenDeck={onOpenDeck}
            onOpenOptions={onOpenOptions}
          />
        ))}
      </div>
    </Card>
  );
}

type DeckRowProps = {
  deck: DeckSummary;
  depth: number;
  onOpenDeck: (deckId: bigint) => void;
  onOpenOptions: (deckId: bigint) => void;
};

function DeckRow({ deck, depth, onOpenDeck, onOpenOptions }: DeckRowProps) {
  const [open, setOpen] = useState(false);
  const hasChildren = deck.children.length > 0;

  // Indent the title cell by one step per level (the count columns stay aligned
  // by the fixed grid, so hierarchy reads as a consistent left edge).
  const titleStyle = { paddingInlineStart: `${depth * 1.25 + 0.75}rem` };

  const title = (
    <div className="flex items-center gap-1.5" style={titleStyle}>
      {hasChildren ? (
        <CollapsibleTrigger
          className="flex size-6 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-border hover:text-foreground"
          onClick={(e: MouseEvent) => e.stopPropagation()}
        >
          <ChevronRight
            className={cn("size-4 transition-transform", open && "rotate-90")}
          />
        </CollapsibleTrigger>
      ) : (
        <span aria-hidden="true" className="size-6 shrink-0" />
      )}

      {deck.studyable ? (
        <button
          type="button"
          onClick={() => onOpenDeck(deck.id)}
          className="truncate text-left text-sm font-medium text-foreground hover:underline"
        >
          {deck.title}
        </button>
      ) : (
        <span
          className="truncate text-sm text-muted-foreground"
          title="Shared for context only"
        >
          {deck.title}
        </span>
      )}
    </div>
  );

  const row = (
    <div
      role="treeitem"
      className={cn(
        "grid min-h-9 items-center gap-2 border-b px-3 py-1.5 last:border-b-0",
        ROW_GRID,
        !deck.studyable && "opacity-60",
      )}
    >
      {title}
      <CountBadge value={deck.new_count} kind="new" />
      <CountBadge value={deck.learning_count} kind="learn" />
      <CountBadge value={deck.review_count} kind="due" />
      <div className="flex justify-end">
        {deck.studyable ? (
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={`Deck options for ${deck.title}`}
            onClick={() => onOpenOptions(deck.id)}
          >
            <Settings />
          </Button>
        ) : null}
      </div>
    </div>
  );

  if (!hasChildren) {
    return row;
  }

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      {row}
      <CollapsibleContent>
        <div role="group">
          {deck.children.map((child) => (
            <DeckRow
              key={String(child.id)}
              deck={child}
              depth={depth + 1}
              onOpenDeck={onOpenDeck}
              onOpenOptions={onOpenOptions}
            />
          ))}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

function CountBadge({ value, kind }: { value: bigint; kind: "new" | "learn" | "due" }) {
  const n = Number(value);
  if (n === 0) {
    return (
      <Badge variant="outline" className="justify-self-center text-muted-foreground">
        {n}
      </Badge>
    );
  }
  const variant =
    kind === "new" ? "default" : kind === "learn" ? "destructive" : undefined;
  return (
    <Badge
      variant={variant ?? "secondary"}
      className={cn(
        "justify-self-center",
        kind === "due" && "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400",
      )}
    >
      {n}
    </Badge>
  );
}
