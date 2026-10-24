import { useEffect, useState } from "react";
import { DeckOptionsEdit } from "shared_types/app";
import type { DeckOptionsView, PresetSummary } from "shared_types/app";
import type { Event } from "shared_types/app";
import {
  eventDeckOptionsListRequested,
  eventDeckOptionsAssign,
  eventDeckOptionsSave,
} from "shared_types/app";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Switch } from "~/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "~/components/ui/select";

// --- Enum option lists (wire snake_case value, human label) ---
const NEW_GATHER_OPTIONS = [
  ["deck", "Deck"],
  ["deck_then_random_notes", "Deck, then random notes"],
  ["ascending", "Ascending position"],
  ["descending", "Descending position"],
  ["random_notes", "Random notes"],
  ["random_cards", "Random cards"],
] as const;
const NEW_SORT_OPTIONS = [
  ["card_type_then_gathered", "Card type, then order gathered"],
  ["gathered", "Order gathered"],
  ["card_type_then_random", "Card type, then random"],
  ["random_note_then_card_type", "Random note, then card type"],
  ["random", "Random"],
] as const;
const NEW_REVIEW_OPTIONS = [
  ["mix", "Mix with reviews"],
  ["after", "Show after reviews"],
  ["before", "Show before reviews"],
] as const;
const REVIEW_SORT_OPTIONS = [
  ["due_then_random", "Due date, then random"],
  ["due_then_deck", "Due date, then deck"],
  ["deck_then_due", "Deck, then due date"],
  ["ascending_interval", "Ascending intervals"],
  ["descending_interval", "Descending intervals"],
  ["easy_first", "Easy first"],
  ["difficult_first", "Difficult first"],
  ["ascending_retrievability", "Ascending retrievability"],
  ["descending_retrievability", "Descending retrievability"],
  ["relative_overdueness", "Relative overdueness"],
  ["random", "Random"],
  ["order_added", "Order added"],
  ["latest_added_first", "Latest added first"],
] as const;
const INSERTION_ORDER_OPTIONS = [
  ["sequential", "Sequential"],
  ["random", "Random"],
] as const;
const LEECH_ACTION_OPTIONS = [
  ["tag_only", "Tag only"],
  ["suspend_card", "Suspend card"],
] as const;
const QUESTION_ACTION_OPTIONS = [
  ["show_answer", "Show answer"],
  ["show_card", "Show card"],
] as const;
const ANSWER_ACTION_OPTIONS = [
  ["bury_card", "Bury card"],
  ["answer_again", "Answer again"],
  ["answer_good", "Answer good"],
  ["answer_hard", "Answer hard"],
  ["show_reminder", "Show reminder"],
] as const;
const EASY_DAY_OPTIONS = [
  ["minimum", "Minimum"],
  ["reduced", "Reduced"],
  ["normal", "Normal"],
] as const;
const EASY_DAY_NAMES = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"] as const;

// --- Formatting helpers (mirror the Leptos impl) ---
function formatDesiredRetention(value: number): string {
  return `${Math.round(value * 100)}`;
}
function formatFloatInput(value: number): string {
  return Number.isInteger(value) ? `${value}` : value.toFixed(1);
}
function formatIntervalSeconds(seconds: bigint): string {
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
function formatSteps(steps: bigint[]): string {
  return steps.length === 0 ? "(none)" : steps.map(formatIntervalSeconds).join(" ");
}
function formatParameters(params: number[]): string {
  return params.map((w) => w.toFixed(4)).join(", ");
}

// --- Editable draft state (mirrors DeckOptionsEditState) ---
interface Draft extends Record<string, unknown> {
  new_per_day: string;
  review_per_day: string;
  leech_threshold: string;
  maximum_answer_seconds: string;
  maximum_interval: string;
  desired_retention: string;
  auto_advance_show_question: string;
  auto_advance_show_answer: string;
  learning_steps: string;
  relearning_steps: string;
  bury_new: boolean;
  bury_review: boolean;
  bury_interday: boolean;
  dont_play_audio_automatically: boolean;
  skip_question_when_replaying_answer: boolean;
  show_on_screen_timer: boolean;
  stop_timer_on_answer: boolean;
  auto_advance_wait_for_audio: boolean;
  new_gather_order: string;
  new_sort_order: string;
  new_review_order: string;
  interday_order: string;
  review_sort_order: string;
  insertion_order: string;
  leech_action: string;
  auto_advance_question_action: string;
  auto_advance_answer_action: string;
  easy_days: string[];
}

function draftFromView(o: DeckOptionsView): Draft {
  return {
    new_per_day: String(o.new_per_day),
    review_per_day: String(o.review_per_day),
    leech_threshold: String(o.leech_threshold),
    maximum_answer_seconds: String(o.maximum_answer_seconds),
    maximum_interval: String(o.maximum_interval),
    desired_retention: formatDesiredRetention(o.desired_retention),
    auto_advance_show_question: formatFloatInput(o.auto_advance_seconds_show_question),
    auto_advance_show_answer: formatFloatInput(o.auto_advance_seconds_show_answer),
    learning_steps: formatSteps(o.learning_steps),
    relearning_steps: formatSteps(o.relearning_steps),
    bury_new: o.bury_new,
    bury_review: o.bury_review,
    bury_interday: o.bury_interday,
    dont_play_audio_automatically: o.dont_play_audio_automatically,
    skip_question_when_replaying_answer: o.skip_question_when_replaying_answer,
    show_on_screen_timer: o.show_on_screen_timer,
    stop_timer_on_answer: o.stop_timer_on_answer,
    auto_advance_wait_for_audio: o.auto_advance_wait_for_audio,
    new_gather_order: o.new_gather_order,
    new_sort_order: o.new_sort_order,
    new_review_order: o.new_review_order,
    interday_order: o.interday_order,
    review_sort_order: o.review_sort_order,
    insertion_order: o.insertion_order,
    leech_action: o.leech_action,
    auto_advance_question_action: o.auto_advance_question_action,
    auto_advance_answer_action: o.auto_advance_answer_action,
    easy_days: Array.from({ length: 7 }, (_, i) => o.easy_days[i] ?? "normal"),
  };
}

// Parse the draft into a DeckOptionsEdit; returns { edit } on success or { error }.
function draftToEdit(draft: Draft): { edit: DeckOptionsEdit } | { error: string } {
  const int = (s: string, what: string): number | string => {
    const t = s.trim();
    const n = Number(t);
    return Number.isInteger(n) && t !== "" ? n : `${what} must be a whole number`;
  };
  const float = (s: string, what: string): number | string => {
    const t = s.trim();
    const n = Number(t);
    return t !== "" && Number.isFinite(n) ? n : `${what} must be a number`;
  };

  const new_per_day = int(draft.new_per_day, "New cards/day");
  if (typeof new_per_day === "string") return { error: new_per_day };
  const review_per_day = int(draft.review_per_day, "Maximum reviews/day");
  if (typeof review_per_day === "string") return { error: review_per_day };
  const leech_threshold = int(draft.leech_threshold, "Leech threshold");
  if (typeof leech_threshold === "string") return { error: leech_threshold };
  const maximum_answer_seconds = int(draft.maximum_answer_seconds, "Maximum answer seconds");
  if (typeof maximum_answer_seconds === "string") return { error: maximum_answer_seconds };
  const maximum_interval = int(draft.maximum_interval, "Maximum interval");
  if (typeof maximum_interval === "string") return { error: maximum_interval };
  const desired_retention = float(draft.desired_retention, "Desired retention");
  if (typeof desired_retention === "string") return { error: desired_retention };
  const aaq = float(draft.auto_advance_show_question, "Auto-advance seconds (question)");
  if (typeof aaq === "string") return { error: aaq };
  const aaa = float(draft.auto_advance_show_answer, "Auto-advance seconds (answer)");
  if (typeof aaa === "string") return { error: aaa };

  return {
    edit: new DeckOptionsEdit(
      null,
      draft.learning_steps.trim(),
      draft.relearning_steps.trim(),
      desired_retention / 100,
      draft.bury_new,
      draft.bury_review,
      draft.bury_interday,
      BigInt(new_per_day),
      BigInt(review_per_day),
      BigInt(leech_threshold),
      BigInt(maximum_answer_seconds),
      BigInt(maximum_interval),
      draft.leech_action,
      draft.new_gather_order,
      draft.new_sort_order,
      draft.new_review_order,
      draft.interday_order,
      draft.review_sort_order,
      draft.insertion_order,
      draft.show_on_screen_timer,
      draft.stop_timer_on_answer,
      draft.dont_play_audio_automatically,
      draft.skip_question_when_replaying_answer,
      aaq,
      aaa,
      draft.auto_advance_wait_for_audio,
      draft.auto_advance_question_action,
      draft.auto_advance_answer_action,
      draft.easy_days,
      null,
    ),
  };
}

type DeckOptionsScreenProps = {
  deckId: bigint | null;
  options: DeckOptionsView | null;
  error: string | null;
  presets: PresetSummary[];
  presetsError: string | null;
  canManagePresets: boolean;
  update: (event: Event) => void;
};

export function DeckOptionsScreen({
  deckId,
  options,
  error,
  presets,
  presetsError,
  canManagePresets,
  update,
}: DeckOptionsScreenProps) {
  const [draft, setDraft] = useState<Draft | null>(null);
  const [localError, setLocalError] = useState<string | null>(null);

  // Initialize the draft once the options load (and on new options id).
  useEffect(() => {
    setDraft(options ? draftFromView(options) : null);
    setLocalError(null);
  }, [options?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  // US-4.11: request the preset list once on entry.
  useEffect(() => {
    update(eventDeckOptionsListRequested());
  }, [update]);

  if (!options) {
    return (
      <div className="h-full overflow-y-auto p-4">
        {error ? (
          <p className="text-destructive">Could not load deck options: {error}</p>
        ) : (
          <p className="text-muted-foreground">No deck options to show.</p>
        )}
      </div>
    );
  }

  const readOnly = options.id === 0n;
  const canEdit = canManagePresets && !readOnly;

  function set<K extends keyof Draft>(key: K, value: Draft[K]) {
    setDraft((d) => (d ? { ...d, [key]: value } : d));
  }
  function setEasyDay(i: number, value: string) {
    setDraft((d) => {
      if (!d) return d;
      const easy_days = d.easy_days.slice();
      easy_days[i] = value;
      return { ...d, easy_days };
    });
  }

  function onSave() {
    if (!draft) return;
    setLocalError(null);
    const result = draftToEdit(draft);
    if ("error" in result) {
      setLocalError(result.error);
    } else {
      update(eventDeckOptionsSave(result.edit));
    }
  }

  return (
    <div className="h-full overflow-y-auto p-4">
      <div className="mx-auto flex max-w-2xl flex-col gap-6">
        <header className="flex items-baseline gap-2">
          <h2 className="text-base font-semibold">Deck options</h2>
          <span className="text-sm text-muted-foreground">
            {options.name}
            {readOnly ? " — read-only" : ""}
          </span>
        </header>

        {readOnly ? (
          <p className="text-sm text-muted-foreground">
            This is the shared default preset. It can&apos;t be edited — create your own
            preset to customise scheduling.
          </p>
        ) : !canManagePresets ? (
          <p className="text-sm text-muted-foreground">
            You don&apos;t have permission to change deck options.
          </p>
        ) : null}

        {canManagePresets ? (
          <div className="flex flex-col gap-2">
            <span className="text-sm font-medium">Preset</span>
            <PresetAssign
              currentId={options.id}
              presets={presets}
              deckId={deckId}
              update={update}
            />
            {presetsError ? (
              <span className="text-destructive">{presetsError}</span>
            ) : null}
          </div>
        ) : null}

        {draft ? (
          <>
            <OptionGroup title="Daily limits">
              <NumberField label="New cards/day" value={draft.new_per_day} onChange={(v) => set("new_per_day", v)} />
              <NumberField label="Maximum reviews/day" value={draft.review_per_day} onChange={(v) => set("review_per_day", v)} />
            </OptionGroup>

            <OptionGroup title="New cards">
              <TextField label="Learning steps" value={draft.learning_steps} onChange={(v) => set("learning_steps", v)} placeholder="1m 10m" />
              <SelectField label="Insertion order" value={draft.insertion_order} options={INSERTION_ORDER_OPTIONS} onChange={(v) => set("insertion_order", v)} />
            </OptionGroup>

            <OptionGroup title="Lapses">
              <TextField label="Relearning steps" value={draft.relearning_steps} onChange={(v) => set("relearning_steps", v)} placeholder="10m" />
              <NumberField label="Leech threshold" value={draft.leech_threshold} onChange={(v) => set("leech_threshold", v)} />
              <SelectField label="Leech action" value={draft.leech_action} options={LEECH_ACTION_OPTIONS} onChange={(v) => set("leech_action", v)} />
            </OptionGroup>

            <OptionGroup title="Display order">
              <SelectField label="New card gather order" value={draft.new_gather_order} options={NEW_GATHER_OPTIONS} onChange={(v) => set("new_gather_order", v)} />
              <SelectField label="New card sort order" value={draft.new_sort_order} options={NEW_SORT_OPTIONS} onChange={(v) => set("new_sort_order", v)} />
              <SelectField label="New/review order" value={draft.new_review_order} options={NEW_REVIEW_OPTIONS} onChange={(v) => set("new_review_order", v)} />
              <SelectField label="Interday learning/review order" value={draft.interday_order} options={NEW_REVIEW_OPTIONS} onChange={(v) => set("interday_order", v)} />
              <SelectField label="Review sort order" value={draft.review_sort_order} options={REVIEW_SORT_OPTIONS} onChange={(v) => set("review_sort_order", v)} />
            </OptionGroup>

            <OptionGroup title="Burying">
              <ToggleField label="Bury new siblings" checked={draft.bury_new} onChange={(v) => set("bury_new", v)} />
              <ToggleField label="Bury review siblings" checked={draft.bury_review} onChange={(v) => set("bury_review", v)} />
              <ToggleField label="Bury interday learning siblings" checked={draft.bury_interday} onChange={(v) => set("bury_interday", v)} />
            </OptionGroup>

            <OptionGroup title="Audio">
              <ToggleField label="Don't play audio automatically" checked={draft.dont_play_audio_automatically} onChange={(v) => set("dont_play_audio_automatically", v)} />
              <ToggleField label="Skip question when replaying answer" checked={draft.skip_question_when_replaying_answer} onChange={(v) => set("skip_question_when_replaying_answer", v)} />
            </OptionGroup>

            <OptionGroup title="Timers">
              <NumberField label="Maximum answer seconds" value={draft.maximum_answer_seconds} onChange={(v) => set("maximum_answer_seconds", v)} />
              <ToggleField label="Show on-screen timer" checked={draft.show_on_screen_timer} onChange={(v) => set("show_on_screen_timer", v)} />
              <ToggleField label="Stop on-screen timer on answer" checked={draft.stop_timer_on_answer} onChange={(v) => set("stop_timer_on_answer", v)} />
            </OptionGroup>

            <OptionGroup title="Auto advance">
              <NumberField label="Seconds to show question" value={draft.auto_advance_show_question} onChange={(v) => set("auto_advance_show_question", v)} />
              <NumberField label="Seconds to show answer" value={draft.auto_advance_show_answer} onChange={(v) => set("auto_advance_show_answer", v)} />
              <ToggleField label="Wait for audio" checked={draft.auto_advance_wait_for_audio} onChange={(v) => set("auto_advance_wait_for_audio", v)} />
              <SelectField label="Question action" value={draft.auto_advance_question_action} options={QUESTION_ACTION_OPTIONS} onChange={(v) => set("auto_advance_question_action", v)} />
              <SelectField label="Answer action" value={draft.auto_advance_answer_action} options={ANSWER_ACTION_OPTIONS} onChange={(v) => set("auto_advance_answer_action", v)} />
            </OptionGroup>

            <OptionGroup title="FSRS">
              <NumberField label="Desired retention (%)" value={draft.desired_retention} onChange={(v) => set("desired_retention", v)} />
              <EasyDaysField days={draft.easy_days} onChange={setEasyDay} />
            </OptionGroup>

            <OptionGroup title="Advanced">
              <NumberField label="Maximum interval (days)" value={draft.maximum_interval} onChange={(v) => set("maximum_interval", v)} />
              <FieldRow label="FSRS parameters">
                <span className="text-muted-foreground">
                  {options.fsrs_parameters.length === 0
                    ? "Default"
                    : formatParameters(options.fsrs_parameters)}
                </span>
              </FieldRow>
            </OptionGroup>

            <footer className="flex items-center gap-2">
              {localError ? (
                <p className="text-sm text-destructive">{localError}</p>
              ) : error ? (
                <p className="text-sm text-destructive">{error}</p>
              ) : null}
              {canEdit ? <Button onClick={onSave}>Save</Button> : null}
            </footer>
          </>
        ) : null}
      </div>
    </div>
  );
}

function PresetAssign({
  currentId,
  presets,
  deckId,
  update,
}: {
  currentId: bigint;
  presets: PresetSummary[];
  deckId: bigint | null;
  update: (event: Event) => void;
}) {
  const items = [
    { label: "Default", value: "0" },
    ...presets.map((p) => ({ label: p.name, value: String(p.id) })),
  ];
  return (
    <Select
      items={items}
      value={String(currentId)}
      onValueChange={(v) => {
        if (v !== null && deckId !== null) {
          update(eventDeckOptionsAssign(deckId, BigInt(v)));
        }
      }}
    >
      <SelectTrigger className="w-full">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          {items.map((item) => (
            <SelectItem key={item.value} value={item.value}>
              {item.label}
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  );
}

function OptionGroup({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-2">
      <h3 className="text-sm font-semibold">{title}</h3>
      <div className="flex flex-col gap-2">{children}</div>
    </div>
  );
}

function FieldRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <span className="text-sm text-muted-foreground">{label}</span>
      <div className="flex min-w-[12rem] justify-end">{children}</div>
    </div>
  );
}

function NumberField({ label, value, onChange }: { label: string; value: string; onChange: (v: string) => void }) {
  return (
    <FieldRow label={label}>
      <Input type="number" value={value} onChange={(e) => onChange(e.target.value)} className="w-[12rem]" />
    </FieldRow>
  );
}

function TextField({ label, value, onChange, placeholder }: { label: string; value: string; onChange: (v: string) => void; placeholder?: string }) {
  return (
    <FieldRow label={label}>
      <Input value={value} onChange={(e) => onChange(e.target.value)} placeholder={placeholder} className="w-[12rem]" />
    </FieldRow>
  );
}

function ToggleField({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <FieldRow label={label}>
      <Switch checked={checked} onCheckedChange={onChange} />
    </FieldRow>
  );
}

function SelectField({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: readonly (readonly [string, string])[];
  onChange: (v: string) => void;
}) {
  const items = options.map(([value, label]) => ({ value, label }));
  const currentLabel = items.find((i) => i.value === value)?.label ?? value;
  return (
    <FieldRow label={label}>
      <Select items={items} value={value} onValueChange={(v) => { if (v !== null) onChange(v); }}>
        <SelectTrigger className="w-[12rem]">
          <SelectValue>{currentLabel}</SelectValue>
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            {items.map((item) => (
              <SelectItem key={item.value} value={item.value}>
                {item.label}
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
    </FieldRow>
  );
}

function EasyDaysField({ days, onChange }: { days: string[]; onChange: (i: number, v: string) => void }) {
  return (
    <div className="flex flex-wrap gap-2">
      {EASY_DAY_NAMES.map((name, i) => (
        <div key={name} className="flex flex-col gap-1">
          <span className="text-xs text-muted-foreground">{name}</span>
          <Select
            items={EASY_DAY_OPTIONS.map(([value, label]) => ({ value, label }))}
            value={days[i] ?? "normal"}
            onValueChange={(v) => { if (v !== null) onChange(i, v); }}
          >
            <SelectTrigger className="w-[6rem]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                {EASY_DAY_OPTIONS.map(([value, label]) => (
                  <SelectItem key={value} value={value}>
                    {label}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
        </div>
      ))}
    </div>
  );
}
