import { useMemo, useState } from "react";
import type { StudyCardView, StudyCountsView } from "shared_types/app";
import {
  eventAnswer,
  eventStartStudy,
  eventBuryCard,
  eventBuryNote,
  eventSuspendCard,
  eventSuspendNote,
  eventUnsuspendCard,
  eventUnburyCard,
  eventRescheduleCard,
  eventFlagCard,
  eventResetCard,
  eventNoteDelete,
  eventNoteDetailRequested,
} from "shared_types/app";
import type { Event } from "shared_types/app";
import { cn } from "~/lib/utils";
import { useTheme } from "~/lib/theme-provider";
import { Button } from "~/components/ui/button";
import { Badge } from "~/components/ui/badge";
import { Input } from "~/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "~/components/ui/dropdown-menu";
import { EllipsisVertical, Flag } from "lucide-react";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "~/components/ui/select";

// Anki flag colours (1..=7), plus "no flag". The icon uses `fill-current` so it
// follows `currentColor` — matching the normal shadcn SelectItem behaviour, where
// the parent recolours its descendants to accent-foreground on hover/focus.
const FLAG_OPTIONS: { value: string; label: string; color?: string }[] = [
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

type StudyScreenProps = {
  card: StudyCardView | null;
  counts: StudyCountsView;
  busy: boolean;
  studyError?: string | null;
  actionError?: string | null;
  update: (event: Event) => void;
};

export function StudyScreen({ card, counts, busy, studyError, actionError, update }: StudyScreenProps) {
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      {studyError ? (
        <p className="px-6 pt-4 text-sm text-destructive">{studyError}</p>
      ) : null}
      {actionError ? (
        <p className="px-6 pt-4 text-sm text-destructive">{actionError}</p>
      ) : null}

      <div className="flex min-h-0 flex-1 flex-col">
        {card ? (
          <StudyCard key={String(card.card_id)} card={card} update={update} />
        ) : busy ? (
          <p className="p-6 text-muted-foreground">Loading…</p>
        ) : (
          <StudyDone update={update} />
        )}
      </div>
    </div>
  );
}

// The footer count badges shown in the header, next to the breadcrumb.
export function StudyCountBadges({ counts }: { counts: StudyCountsView }) {
  return (
    <div className="flex items-center gap-1.5">
      <Badge className="bg-blue-500/15 text-blue-600 dark:text-blue-400">
        {counts.new_count.toString()}
      </Badge>
      <Badge variant="destructive">
        {(counts.learning_count + counts.relearning_count).toString()}
      </Badge>
      <Badge className="bg-emerald-500/15 text-emerald-600 dark:text-emerald-400">
        {counts.review_count.toString()}
      </Badge>
    </div>
  );
}

function StudyDone({ update }: { update: (event: Event) => void }) {
  return (
    <div className="flex items-center justify-center p-6">
      <div className="flex flex-col items-start gap-3">
        <p className="text-muted-foreground">Nothing due right now. Nice work!</p>
        <Button onClick={() => update(eventStartStudy())}>Check again</Button>
      </div>
    </div>
  );
}

// A single study card: front (then revealed back) in a sandboxed iframe, with
// "Show answer" / answer buttons pinned to the bottom.
function StudyCard({ card, update }: { card: StudyCardView; update: (event: Event) => void }) {
  const [revealed, setRevealed] = useState(false);
  const { resolvedTheme } = useTheme();
  const isDark = resolvedTheme === "dark";

  const intervals = useMemo(() => card.predicted_interval ?? new Map(), [card.predicted_interval]);
  const intervalFor = (r: number): string => {
    const secs = intervals.get(r);
    return secs === undefined ? "" : formatInterval(secs);
  };

  const srcdoc = useMemo(
    () => cardHtml(revealed ? card.back : card.front, card.styling, isDark),
    [revealed, card.back, card.front, card.styling, isDark],
  );

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <iframe
        className="w-full min-h-0 flex-1 border-none bg-white dark:bg-gray-950"
        sandbox=""
        srcDoc={srcdoc}
        title="Study card"
      />
      <div className="flex shrink-0 justify-center gap-2 px-6 py-4">
        {revealed ? (
          <div className="flex flex-wrap justify-center gap-2">
            <AnswerButton label="Again" interval={intervalFor(1)} variant="destructive" onAnswer={() => update(eventAnswer(1))} />
            <AnswerButton label="Hard" interval={intervalFor(2)} onAnswer={() => update(eventAnswer(2))} />
            <AnswerButton label="Good" interval={intervalFor(3)} onAnswer={() => update(eventAnswer(3))} />
            <AnswerButton label="Easy" interval={intervalFor(4)} onAnswer={() => update(eventAnswer(4))} />
          </div>
        ) : (
          <Button variant="secondary" onClick={() => setRevealed(true)}>
            Show answer
          </Button>
        )}
      </div>
    </div>
  );
}

function AnswerButton({
  label,
  interval,
  variant = "ghost",
  onAnswer,
}: {
  label: string;
  interval: string;
  variant?: "ghost" | "destructive";
  onAnswer: () => void;
}) {
  return (
    <div className="flex flex-col items-center gap-1">
      <Button variant={variant} size="lg" onClick={onAnswer}>
        {label}
      </Button>
      <span className="text-xs text-muted-foreground">{interval}</span>
    </div>
  );
}

// Study-card actions (bury/suspend card+note, reset, reschedule) as a "⋮" dropdown,
// plus a flag select next to it. Reschedule opens a dialog for the day offset.
export function StudyControls({
  card,
  canManageNotes,
  update,
  onDuplicateNote,
}: {
  card: StudyCardView;
  canManageNotes: boolean;
  update: (event: Event) => void;
  onDuplicateNote: () => void;
}) {
  const [rescheduleOpen, setRescheduleOpen] = useState(false);
  const [days, setDays] = useState("");
  const [deleteOpen, setDeleteOpen] = useState(false);

  const flag = String(card.flag);

  function confirmReschedule() {
    const d = Number(days.trim());
    if (Number.isFinite(d)) {
      update(eventRescheduleCard(BigInt(d)));
      setDays("");
      setRescheduleOpen(false);
    }
  }

  function confirmDeleteNote() {
    update(eventNoteDelete(card.note_id));
    setDeleteOpen(false);
  }

  return (
    <div className="flex items-center gap-1">
      <Select
        items={FLAG_OPTIONS}
        value={flag}
        onValueChange={(v) => update(eventFlagCard(Number(v)))}
      >
        <SelectTrigger className="w-[9rem]" aria-label="Flag">
          <Flag
            className={cn("size-4", flag !== "0" && flagColor(flag), flag !== "0" && "fill-current")}
          />
          <SelectValue placeholder="Flags" />
        </SelectTrigger>
        <SelectContent alignItemWithTrigger={false}>
          <SelectGroup>
            <SelectLabel>Flags</SelectLabel>
            {FLAG_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                <Flag
                  className={cn("size-4", opt.color, opt.value !== "0" && "fill-current")}
                />
                {opt.label}
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>

      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <Button variant="ghost" size="icon-sm" aria-label="Card actions" />
          }
        >
          <EllipsisVertical />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="w-44">
          <DropdownMenuGroup>
            <DropdownMenuLabel>Card Actions</DropdownMenuLabel>
            <DropdownMenuItem onClick={() => update(card.buried ? eventUnburyCard() : eventBuryCard())}>
              {card.buried ? "Unbury Card" : "Bury Card"}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => update(card.suspended ? eventUnsuspendCard() : eventSuspendCard())}
            >
              {card.suspended ? "Unsuspend Card" : "Suspend Card"}
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => setRescheduleOpen(true)}>Reschedule</DropdownMenuItem>
            <DropdownMenuItem onClick={() => update(eventResetCard())}>Reset Card</DropdownMenuItem>
          </DropdownMenuGroup>
          <DropdownMenuSeparator />
          <DropdownMenuGroup>
            <DropdownMenuLabel>Note Actions</DropdownMenuLabel>
            <DropdownMenuItem onClick={() => update(eventBuryNote())}>Bury Note</DropdownMenuItem>
            <DropdownMenuItem onClick={() => update(eventSuspendNote())}>Suspend Note</DropdownMenuItem>
            <DropdownMenuItem onClick={onDuplicateNote} disabled={!canManageNotes}>Duplicate Note</DropdownMenuItem>
            <DropdownMenuItem onClick={() => setDeleteOpen(true)} disabled={!canManageNotes}>Delete Note</DropdownMenuItem>
          </DropdownMenuGroup>
        </DropdownMenuContent>
      </DropdownMenu>

      <Dialog open={rescheduleOpen} onOpenChange={setRescheduleOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Reschedule</DialogTitle>
            <DialogDescription>How many days from now?</DialogDescription>
          </DialogHeader>
          <Input
            type="number"
            placeholder="days"
            value={days}
            onChange={(e) => setDays(e.target.value)}
          />
          <DialogFooter>
            <Button variant="outline" onClick={() => setRescheduleOpen(false)}>
              Cancel
            </Button>
            <Button onClick={confirmReschedule}>Reschedule</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

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
            <Button variant="destructive" onClick={confirmDeleteNote}>Delete</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

// Built-in night-mode base (Anki parity). Applied on top of the author's
// `styling` so every card gets a dark canvas + light foreground when the app is
// dark, even if the note type defines no `.nightMode` rule. Only LaTeX and
// drawings are inverted — never arbitrary images.
//
// `body.nightMode .card` (specificity 0,1,1) beats the seed note type's plain
// `.card { background-color: white; color: black }` (0,1,0) so the default card
// doesn't stay white — while an author's explicit `.card.nightMode` /
// `.nightMode …` override (higher specificity) still wins.
const NIGHT_MODE_BASE = `
body.nightMode { background-color: #1f1f1f; color: #e5e5e5; }
body.nightMode .card { background-color: #1f1f1f; color: #e5e5e5; }
.nightMode .latex { filter: invert(100%); }
.nightMode img.drawing { filter: invert(1) hue-rotate(180deg); }
`;

function cardHtml(body: string, styling: string, nightMode: boolean): string {
  const bodyClass = nightMode ? ' class="nightMode"' : "";
  const nightCss = nightMode ? `<style>${NIGHT_MODE_BASE}</style>` : "";
  return `<!DOCTYPE html><html><head><meta charset="utf-8" /><style>${styling}</style>${nightCss}</head><body${bodyClass}>${body}</body></html>`;
}

// Anki interval-unit notation: <1m → "Ns", minutes, hours, days, months, years.
export function formatInterval(seconds: bigint): string {
  const s = Number(seconds);
  const MINUTE = 60;
  const HOUR = 60 * MINUTE;
  const DAY = 24 * HOUR;
  const MONTH = 30 * DAY;
  const YEAR = 365 * DAY;

  if (s < MINUTE) return `${s}s`;
  if (s < HOUR) return `${Math.floor(s / MINUTE)}m`;
  if (s < DAY) return `${Math.floor(s / HOUR)}h`;
  if (s < MONTH) return `${Math.floor(s / DAY)}d`;
  if (s < YEAR) return `${Math.floor(s / MONTH)}mo`;
  return `${Math.floor(s / YEAR)}y`;
}
