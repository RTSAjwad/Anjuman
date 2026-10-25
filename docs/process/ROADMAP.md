# ROADMAP.md

The ordered, single source of truth for what's next. Work these **top to
bottom**: each task is a prerequisite (or strong dependency) for the ones
below it.

Legend:

- `[ ]` not started
- `[~]` in progress
- `[x]` done

---

## 1. Migrate SQLite → Postgres   `[x]`

Done. The server runs on Postgres (TIMESTAMPTZ, JSONB, ENUM types, BOOLEAN)
via a consolidated `0001_create_schema.sql` + `0002_seed_data.sql`, provisioned
by `services-flake` (`nix run .#anjuman`). All handler SQL + the full
`DateTime<Utc>` timestamp migration landed; `cargo build`/`test` pass and
OpenAPI route coverage is unchanged (71 operations).

> Detailed plans: [`docs/adr/postgres-migration.md`](../adr/postgres-migration.md), [`docs/adr/timestamptz-migration.md`](../adr/timestamptz-migration.md).

### Sub-tasks (summary)

- [x] **Phase 0** — Nix-idiomatic Postgres runtime (`flake-parts` +
      `services-flake`; `nix run .#anjuman`).
- [x] **Phase 1** — sqlx `postgres` feature; `db.rs` `PgPool`; type renames.
- [x] **Phase 2** — `0001_create_schema.sql` consolidated schema.
- [x] **Phase 3** — `0002_seed_data.sql`.
- [x] **Phase 4** — handler SQL conversion + full `TIMESTAMPTZ`/
      `JSONB`/`ENUM`/`bool` migration.
- [x] **Phase 5** — flake/docs/cleanup; OpenAPI coverage unchanged.

**Definition of done:** ✅ server boots against Postgres, all 71 endpoints
work, migrations run cleanly from empty, SQLite footprint removed.

---

## 2. Deck options — feature-complete with Anki   `[~]`

> **Scope: server vs. client behaviour.** Some deck options have no server-side
> effect — the server only stores and returns the selection, and the behaviour is
> a client concern (stage 4). Examples: on-screen timer, audio, auto-advance.
> When implementing one of these now, we add the field to the contract + CRUD
> plumbing + preserve it in OpenAPI, and test only its **round-trip through
> create/update/read** — we do *not* implement (or test) the behaviour, and we
> document it as "behaviour client-side" with a stage-4 reference. Full details
> and per-row classification live in [`docs/support/deck-options.md`](../support/deck-options.md) ("Scope: server
> vs. client behaviour").

Close the gaps in [`docs/support/deck-options.md`](../support/deck-options.md) as far as practical. The
matrix's "Summary of the biggest gaps" is the working list:

- [x] **Display order** — US-2.8–US-2.12 (gather, sort, new/review, interday,
      review-sort) all done. See below.
- [x] **FSRS parameter optimization** → US-2.18a (store+consume, done) + US-2.18b (optimizer, **deferred** — needs search engine + memory-state reconstruction + review-kind markers; see story).
- [ ] **Collection-wide FSRS toggles (descoped to stage 7)** — "Reschedule cards
      on change" (transient, not saved) and "Check health when optimizing" are
      both collection-wide booleans in-app; deferred with the other
      collection-wide options (see the daily-limits note below).
- [ ] **FSRS simulator / Help Me Decide (descoped to a post-client stage)** —
      the "FSRS Simulator (Experimental)" and "Help Me Decide (Experimental)"
      buttons each open a distinct simulator (different graphs). Both are UI-
      heavy (and may need server-side simulation endpoints), so descoped until
      after the client lands (stage 4); no server work now.
- [x] **Maximum answer seconds (server-side cap)** → US-2.15 (done).
- [x] **Easy Days (server-side scheduling)** → US-2.16 (done, persist-only; scheduling effect descoped).
- [x] **Maximum interval (server-side cap)** → US-2.17 (done).
- [ ] **Historical retention** → folded into US-2.18.
- [ ] **Custom scheduling (JS) — descoped (collection-wide)** — text-area JS
      hook, collection-wide, "use at your own risk"; deferred with the other
      collection-wide options (stage 7 note below).
- [x] **Deck sharing: subtree access model** → US-2.19 (done) — a grant on a
      deck covers its subtree, ancestors are read-only context, and the
      student's list stays a tree. Added `DeckResponse.studyable` + recursive
      access + `server/tests/deck_sharing.rs`.
- [x] **Leech action (per-student state)** — the leech *action* completes as
      US-2.20 (per-student `leech_at`, not a tag). Threshold + empty relearning
      steps are already done (US-2.2–US-2.4); minimum interval is SM-2 — out
      of scope.
- [x] **US-2.2 — Leech threshold**

      **As** a student failing a review card repeatedly,
      **I want** the card to be flagged a leech after N review-card lapses,
      **so that** I can spot time-sink cards.

      **Acceptance criteria**
      - [x] `lapses` counts review-card "Again" (state Review) only, not
            learning/relearning "Again".
      - [x] Default leech threshold is 8.
      - [x] Threshold configurable via deck options (`leech_threshold`).
      - [x] Each criterion has a `server/tests/` test.

      **Out of scope**
      - Half-threshold re-warnings (no notification system).
- [x] **US-2.3 — Leech action (enum)**

      **As** a student,
      **I want** a leech card to be suspended (configurably),
      **so that** it stops consuming study time until I unsuspend it.

      **Acceptance criteria**
      - [x] `leech_action` is an enum: `TagOnly` | `SuspendCard`.
      - [x] `SuspendCard` suspends the card at threshold (default action).
      - [x] `TagOnly` is accepted on the wire but not yet effective.
      - [x] Each criterion has a `server/tests/` test.

      **Out of scope / documented divergence**
      - Tagging the note (both actions tag in Anki) — Anjuman does **not** model
        leech as a tag. Leech is per-student scheduling state, not a shared note
        tag (US-6.6). `TagOnly` is a no-op until that per-student marker lands.
- [x] **US-2.4 — Empty relearning steps**

      **As** a student who lets FSRS control short-term scheduling,
      **I want** an empty relearning-steps list to skip the relearning phase,
      **so that** FSRS recomputes the interval directly on a lapse.

      **Acceptance criteria**
      - [x] Empty relearning steps are accepted (no "at least one step" error).
      - [x] On `Again` from a review card, the card stays `review` and its
            interval is recomputed via FSRS (no relearning phase).
      - [x] Each criterion has a `server/tests/` test.

      **Out of scope**
      - Empty *learning* steps (see US-2.5).
- [ ] **US-2.5 — Empty learning steps → FSRS short-term scheduling**

      **As** a student who lets FSRS control short-term scheduling,
      **I want** an empty learning-steps list to let FSRS schedule the
      learning phase too,
      **so that** I can drop the fixed step ladder entirely.

      **Acceptance criteria**
      - TBD — needs an `fsrs`-crate investigation first: how does `fsrs 6.6`
        expose FSRS-5 short-term (learning-phase) intervals, and what does
        "Again" mean without a step ladder (can be ≥1 day).
      - [ ] New/learning cards schedule via FSRS rather than `learning_steps`.
      - [ ] Each criterion has a `server/tests/` test.

      **Matches Anki's "experimental" flag** — the manual marks
      "leaving the (re)learning steps field empty" as experimental; we mirror
      that status and defer this behind the non-experimental deck-options gaps.

      **Notes**
      - Requires revisiting whether short-term scheduling is a small branch or
        a real scheduler change (depends on the `fsrs` crate's learning-phase
        API). Place after Display order / parameter optimization / daily limits.
- [ ] **Subdeck limit aggregation** — "selected deck governs the total" across
      a subtree.
- [ ] **Daily-limit fine controls** — implemented as US-2.6 below (per-deck
      `preset`/`this_deck`/`today_only` override on `new_per_day`/`review_per_day`).
- [ ] **`new_cards_ignore_review_limit` + `limits_start_from_top`** — descoped
      until after stage 7. These are "collection-wide" in Anki; our multi-tenant
      school/user split makes their scope a stage-7 decision (see stage 7 design
      question #6).
- [x] **US-2.6 — Per-deck daily-limit overrides**

      **As** a teacher tailoring one deck,
      **I want** a per-deck `preset`/`this deck`/`today only` override on the
      new-cards and review limits,
      **so that** one deck can have its own caps without forking a whole preset.

      **Acceptance criteria**
      - [x] A deck in `preset` mode uses its shared preset's limit (unchanged).
      - [x] `this_deck` overrides the limit for that deck only; sibling decks
            sharing the preset are unaffected.
      - [x] `today_only` applies today and falls back to the preset limit on the
            next study day (lazy expiry).
      - [x] New and review limits are independently selectable (per-limit mode).
      - [x] `this_deck`/`today_only` without a value → `400`.
      - [x] Each criterion has a `server/tests/` test.

      **Out of scope**
      - Subdeck limit aggregation (US-2.7).
      - Stage-7 per-user personalisation (precedence reconciled later).

      **Model**
      - Columns on `decks` (school-authored, teacher/admin): `new_per_day_mode`
        + `review_per_day_mode` (`ENUM preset/this_deck/today_only`, default
        `preset`), nullable `new_per_day_override`/`review_per_day_override`, and
        `new_per_day_today_date`/`review_per_day_today_date` for lazy expiry.
        `deck_options.new_per_day`/`review_per_day` remain the preset base.
- [x] **US-2.7 — Subdeck limit aggregation**

      **As** a student studying a deck with subdecks,
      **I want** each subdeck's own limit to cap gathering from that subdeck,
      while the selected deck's limit caps the session total,
      **so that** limits compose correctly across the deck tree.

      **Acceptance criteria**
      - [x] Gathering honours each subdeck's effective limit (per-subdeck cap).
      - [x] The selected deck's limit caps the overall session total.
      - [x] Each criterion has a `server/tests/` test.

      **Out of scope**
      - `limits_start_from_top` (collection-wide; deferred to after stage 7).
        Base aggregation is algorithm-neutral and built now; the toggle is the
        deferred piece.

      **Relation to US-2.6** — resolves each subdeck's *effective* limit via the
      same `effective_daily_limits` helper.
- [x] **US-2.1 — Hard-button step behaviour** — implement Anki's exact
      learning/relearning Hard rules. See [`docs/process/PLANNING.md`](./PLANNING.md) §2.

      **As** a student reviewing a (re)learning card,
      **I want** the Hard button to follow Anki's step rules,
      **so that** my scheduling matches Anki's expected intervals.

      **Acceptance criteria**
      - [x] On the first learning step, Hard = average of the first two steps.
      - [x] With a single step, Hard = 1.5× that step (capped at +1 day).
      - [x] On any other step, Hard repeats the current step.
      - [x] `predict_intervals` mirrors `apply_review` (frontend hints agree).
      - [x] Each criterion has a `server/tests/` test.

      **Out of scope**
      - Review (graduated) cards: Hard already uses the FSRS interval.
- [x] **US-2.8a — Add `cards.position` (prereq for display order)**

      **As** the scheduler,
      **I want** a per-card position column,
      **so that** "ascending/descending position" and several sort orders are
      meaningful.

      **Acceptance criteria**
      - [x] `cards.position BIGINT NOT NULL` exists, defaulting to creation
            order (backfill = `id`-monotonic) so existing data is ordered.
      - [x] New cards get a monotonic position on insert.
      - [x] Each criterion has a `server/tests/` test (or a migration-verified
            backfill).

      **Out of scope**
      - Manual reordering UI (separate future story).
- [x] **US-2.8 — New card gather order**

      **As** a student,
      **I want** to choose how new cards are gathered (deck / ascending /
      descending / random notes / random cards),
      **so that** I control which new cards are prioritised.

      **Acceptance criteria**
      - [x] `new_gather_order` enum on `deck_options` (default `deck`), incl.
            `deck_then_random_notes`.
      - [x] `deck` gathers subdecks in order, each in ascending position.
      - [x] `deck_then_random_notes` keeps subdeck order but randomises notes
            within each subdeck.
      - [x] `ascending`/`descending` order by `cards.position`.
      - [x] `random_notes`/`random_cards` use a deterministic per-student-day
            seed (see divergence note).
      - [x] Each criterion has a `server/tests/` test.

      **Out of scope**
      - Manual card/note reordering UI (a separate future story).
- [x] **US-2.9 — New card sort order**

      **As** a student,
      **I want** to choose how gathered new cards are sorted (card type /
      gathered / card-type+random / random note+card type / random),
      **so that** sibling cards are spaced the way I prefer.

      **Acceptance criteria**
      - [x] `new_sort_order` enum on `deck_options` (default card-type order).
      - [x] Card-type ordering uses `note_type_templates.ord`.
      - [x] Deterministic per-student-day seed for random variants.
      - [x] Each criterion has a `server/tests/` test.
- [x] **US-2.10 — New/review order**

      **As** a student,
      **I want** to choose whether new cards mix with, precede, or follow
      review cards,
      **so that** I control the study session shape.

      **Acceptance criteria**
      - [x] `new_review_order` enum (mix / before / after), default mix.
      - [x] `before`/`after` are fixed block orderings.
      - [x] `mix` is Anki's stateless `Intersperser` (ratio-based even
            distribution), implemented directly from the day counters.
      - [x] Each criterion has a `server/tests/` test.
- [x] **US-2.11 — Interday learning/review order**

      **As** a student,
      **I want** to choose whether interday (re)learning cards mix with,
      precede, or follow review cards,
      **so that** I can front-load or defer harder cards.

      **Acceptance criteria**
      - [x] `interday_order` enum (mix / before / after), default mix.
      - [x] Interday learning is always *gathered* first (limit applied first),
            but its *display* rank vs review follows the setting.
      - [x] `mix` uses the same stateless `Intersperser` as US-2.10.
      - [x] Each criterion has a `server/tests/` test.
- [x] **US-2.12 — Review sort order**

      **As** a student,
      **I want** to choose the review sort order,
      **so that** I can clear a backlog or prioritise due cards sensibly.

      **Acceptance criteria**
      - [x] `review_sort_order` enum (default `due_then_random`) covering all
            13 in-app options: due-then-random, due-then-deck, deck-then-due,
            ascending/descending interval, easy/difficult first, ascending/
            descending retrievability, relative overdueness, random, order
            added, latest-added first.
      - [x] "interval" maps to FSRS `stability`; "easy/difficult" maps to
            FSRS `difficulty`.
      - [x] "order added"/"latest added" map to `c.id` (creation-order proxy;
            `created_at`/position are distinct concerns).
      - [x] Retrievability orders by the monotonic ratio
            `(stability + overdue)/stability`, which is exactly equivalent to
            `fsrs::current_retrievability` (strictly monotonic in that ratio).
      - [x] Each criterion has a `server/tests/` test.

      **Documented inconsistencies** (also recorded in the support matrix):
      - "Relative overdueness" appears in the in-app list *despite* the manual
        saying it is removed under FSRS; implemented (overdueness = elapsed ÷
        interval).
      - "Ascending/descending ease" (SM-2) is surfaced as easy/difficult-first
        under FSRS via `difficulty`.
- [x] **US-2.13 — New card insertion order (Random + retroactive re-sort)**

      **As** a teacher configuring a preset,
      **I want** to choose whether newly-added cards get sequential or random
      positions (due #) and have that choice re-sort existing new cards,
      **so that** new-card introduction order matches my preference without
      reordering by hand.

      **Acceptance criteria**
      - [x] `insertion_order` enum (`sequential` / `random`) on `deck_options`,
            default `sequential`.
      - [x] Sequential keeps `card_position_seq` monotonic assignment.
      - [x] Random assigns shuffled `position`s to cards created while active
            (on-insert, via `sync_card_rows`).
      - [x] Changing the option atomically re-sorts the *existing* new-card
            `position`s of the preset's decks (retroactive — `random` shuffles,
            `sequential` restores id order).
      - [x] Each criterion has a `server/tests/` test.

      **Resolved scope / divergence (documented)** — Anki's new-card position is
      a *global* `due` namespace (single-user); the source confirms this (no
      per-deck partition). In our multi-tenant system we scope the re-sort to
      the preset's decks — a deliberate divergence from Anki's global namespace
      (which would otherwise let one school's `random` reshuffle other schools'
      new cards). `position` only affects new-card ordering, so renumbering all
      cards of the preset's decks is safe.

      **Out of scope** — the bulk `create_template` path leaves positions
      sequential (rare path; the normal note-creation path is covered).
- [x] **US-2.14 — Selection-only deck options (client-side behaviour)**

      Persist the deck options whose behaviour is purely client-side, so they
      are available on the wire now and consumable when the client implements
      them (stage 4). Server work is **fixed fields + CRUD round-trip only** —
      no behaviour, and tests assert persistence, not effect.

      Options (from [`docs/support/deck-options.md`](../support/deck-options.md) "Audio" / "Timers" / "Auto Advance"):
      - [x] On-screen timer — "Show on-screen timer" (boolean, default off).
      - [x] On-screen timer — "Stop on-screen timer on answer" (boolean, default off).
      - [x] Audio — "Don't play audio automatically" (boolean).
      - [x] Audio — "Skip question when replaying answer" (boolean).
      - [x] Auto advance — "Seconds to show question for" (f64 1dp, 0.0–9999.0, default 0.0).
      - [x] Auto advance — "Seconds to show answer for" (f64 1dp, 0.0–9999.0, default 0.0).
      - [x] Auto advance — "Wait for audio" (boolean, default on).
      - [x] Auto advance — "Question action" (enum `show_answer` | `show_card`, default `show_answer`).
      - [x] Auto advance — "Answer action" (enum `bury_card` | `answer_again` | `answer_good` | `answer_hard` | `show_reminder`, default `bury_card`).

      **Decision** — treated as client-side: the server persists the values only
      (no timing/advancing behaviour).

      **Acceptance criteria** (per option)
      - [x] Field added to the contract + `deck_options` table + CRUD
            create/update/read plumbing.
      - [x] Field appears in the generated OpenAPI spec.
      - [x] Round-trip test: value survives create → read and update → read.
      - [x] Documented "behaviour client-side" with a stage-4 reference.

      **Out of scope** (until stage 4): any actual timer/audio/advance
      behaviour in a client.
- [x] **US-2.15 — Maximum answer seconds (server-side cap)**

      **As** a student,
      **I want** answers that took longer than a per-preset cap to be recorded
      as the cap,
      **so that** my statistics aren't skewed by time I was away from the screen.

      Anki manual: <https://docs.ankiweb.net/deck-options.html#timers> (in-app it
      is a **number, min 1, max 7200**, default 60; the manual's "60s" is the
      default value, not a hard bound).

      **Acceptance criteria**
      - [x] `maximum_answer_seconds` field on `deck_options` (`i64`, default 60,
            validated `1..=7200` — reject `0` and `>7200`, matching Anki).
      - [x] `apply_review` caps the recorded `response_time_ms` at
            `maximum_answer_seconds` when writing the `reviews` row (and the
            pre-submission stats path reads the capped value).
      - [x] `GET /deck-options` reflects the field (OpenAPI regenerated).
      - [x] Each criterion has a `server/tests/` test.

      **Client-side contract (documented, stage 4)**
      - The client runs the stopwatch and sends `response_time_ms` (existing
        `StudyAdvanceBody` field).
      - The client mirrors the `1..=7200` range in its input UI, and *may*
        pre-clamp `response_time_ms` for an accurate on-screen cap — but the
        server's clamp is authoritative and sufficient for correctness.

      **Out of scope**
      - The on-screen timer UI (client, stage 4 — already persisted in US-2.14).
      - Collection-wide cap: this is preset-scoped (in-app it lives under Timers
        in deck options), not a preferences value.
- [x] **US-2.16 — Easy Days (per-weekday workload, persist-only)**

      **As** a teacher configuring a preset,
      **I want** to record per-weekday workload levels on the preset,
      **so that** they are available on the wire for the client (and a future
      scheduler) to act on.

      Anki manual: <https://docs.ankiweb.net/deck-options.html#easy-days>.
      In-app it is **one three-value slider per weekday** (`minimum` / `reduced` /
      `normal`), non-retroactive.

      **Scope** — **persist-only (selection-only).** No scheduling behaviour.
      Anki's Easy Days is not a simple post-hoc shift; it is woven into Anki's
      *interval fuzzer / load balancer* (per-preset day projections over the
      fuzz window ≤ 90 days, with `Reduced` evaluated dynamically via weighted
      random sampling) — a subsystem we do not implement at all. The scheduling
      effect is therefore descoped to a future "interval load balancer" story.

      **Acceptance criteria**
      - [x] `easy_days` field on `deck_options` — one `EasyDayStrength` enum
            (`minimum` / `reduced` / `normal`) per weekday (Mon–Sun, 7 values),
            default all `normal`.
      - [x] `GET /deck-options` reflects the field (OpenAPI regenerated).
      - [x] Round-trip test: value survives create → read and update → read.

      **Out of scope (documented divergence)**
      - Any scheduling effect (the load-balancing in Anki's interval fuzzer);
        deferred to a future "interval load balancer" story.
      - Collection-wide (Anki scopes Easy Days per collection); we scope it
        per-preset, de facto a stage-7 collection-wide decision.
- [x] **US-2.17 — Maximum interval (server-side cap)**

      **As** a student,
      **I want** to cap how far out a review card can be scheduled,
      **so that** my intervals don't grow beyond a bound I choose.

      Anki manual: <https://docs.ankiweb.net/deck-options.html#maximum-interval>.
      In-app it is a **number, default 36500, min 0, max 36500** (at the cap,
      Hard/Good/Easy give the same delay).

      **Acceptance criteria**
      - [x] `maximum_interval` field on `deck_options` (`i64` days, default
            36500, validated `1..=36500` — reject `0` and `>36500`).
      - [x] `apply_review` clamps the computed FSRS interval to
            `maximum_interval` days (stored `stability` stays uncapped).
      - [x] `GET /deck-options` reflects the field (OpenAPI regenerated).
      - [x] Each criterion has a `server/tests/` test.

      **Divergence from Anki (documented)** — Anki's UI permits `0`, but its
      scheduler floors the maximum at 1 day (`maximum.max(1)`), so `0` *behaves
      identically to `1`. We reject `0` and use `1` as the minimum, diverging
      only in *not accepting the redundant input*, not in behaviour.

      **Out of scope**
      - Learning/relearning *step* delays are not capped by maximum interval
        (Anki applies it to the review interval only).
- [x] **US-2.18a — FSRS parameters (store + consume)**

      **As** a teacher,
      **I want** to store FSRS parameters on a preset and have scheduling use
      them,
      **so that** shared decks can be tuned beyond the default parameters.

      Anki manual: <https://docs.ankiweb.net/deck-options.html#fsrs-parameters>.

      **Acceptance criteria**
      - [x] `fsrs_parameters` (JSONB weight vector) field on `deck_options`
            (empty/absent = `FSRS::default()`).
      - [x] `apply_review` uses `FSRS::new(&preset.fsrs_parameters)` (empty →
            default) instead of `FSRS::default()`.
      - [x] `GET /deck-options` reflects the field (OpenAPI regenerated).
      - [x] Each criterion has a `server/tests/` test (incl. one proving stored
            parameters are consumed without error).

      **Out of scope** — the optimizer endpoint (US-2.18b) and
      `historical_retention` (which only feeds the optimizer's gap-filling, so
      it is deferred with it).
- [ ] **US-2.18b — FSRS parameter optimizer (deferred)**

      **As** a teacher,
      **I want** to optimise a preset's FSRS parameters from the school's review
      history (optionally filtered to a search),
      **so that** scheduling fits the class's actual retention.

      Anki manual:
      <https://docs.ankiweb.net/deck-options.html#fsrs-parameters> and
      <https://docs.ankiweb.net/deck-options.html#historical-retention>.

      **Acceptance criteria** (deferred — not yet written/tested)
      - [ ] Optimizer endpoint (`POST /deck-options/:id/optimize`) running
            `fsrs::compute_parameters` over history reconstructed from `reviews`
            (per-card `FSRSItem`s, day deltas), storing the result.
      - [ ] `historical_retention` field (default 0.9, `0.7..=0.97`) feeds the
            optimizer's gap-filling.
      - [ ] Optional `param_search` filter (the in-app
            `preset: "Default" ~is:suspended` box).
      - [ ] `num_relearning_steps` aligned to the preset's relearning steps.
      - [ ] Each criterion has a `server/tests/` test.

      **Deferred (decision recorded 2026-09-29).** The optimizer needs several
      prerequisites our schema/features don't yet have, and is low-value before
      the client generates real review history. Defer until after stage 4 (and
      stage 6's tag/search work). Reason (from Anki source, `rslib/src/scheduler/fsrs/params.rs`):

      1. **`review_kind` + reset/manual markers.** Anki's `reviews_for_fsrs`
         filters out cramming (`Filtered`), manual/set-due (`Manual`), and reset
         (ease==0) entries before building `FSRSItem`s, and classifies
         Learning/Review/Relearning. Our `reviews` table has only `state_before`
         (new/learning/review/relearning), `rating`, and `interday` — no reset
         or manual/filtered marker, so faithful training-data selection is
         impossible without a `reviews` schema change + `apply_review` write
         path additions.
      2. **`param_search` needs a search engine.** The in-app filter
         (`preset: "Default" ~is:suspended`) is a full card-search query over
         the revlog. We have no search engine (stage 6 territory).
      3. **`historical_retention` needs memory-state reconstruction.** In Anki
         it feeds `memory_state.rs` (gap-filling when history is incomplete),
         *not* `compute_parameters` directly. That reconstruction logic doesn't
         exist here, so `historical_retention` can only be *persisted* (like
         US-2.18a), not meaningfully consumed, until it's built.
      4. **"keep-if-better" guard + health check** rely on model evaluation
         (`evaluate`/`evaluate_with_time_series_splits`) and the descoped
         collection-wide "check health" toggle (stage 7).
      5. **No real corpus yet.** The optimizer is only useful with hundreds of
         reviews per preset; we're pre-client, so there's nothing meaningful to
         optimize against.

      **Out of scope / decision needed**
      - "Check health when optimizing" (collection-wide, stage 7) and the
        simulators (post-client).
      - Storage scope is per-preset columns on `deck_options` (Anki: per-preset
        shared across the collection — recorded).
- [x] Cross-check the remaining ❌/🟡 rows and either implement or consciously
      descope each. **Done 2026-09-29.** Every remaining non-✅ row now carries a
      descope/deferral note: the ⏳ collection-wide FSRS options (stage 7), the
      post-client simulators, `historical_retention`/`ignore_before` (with
      US-2.18b), custom scheduling (stage 7), desired-retention per-deck
      (stage 7), leech action (US-2.20, per-student state — not a tag), Easy
      Days scheduling (interval load balancer), US-2.5 (empty learning steps,
      experimental), US-2.18b (optimizer). Fixed two stale rows: "Display
      order taken from selected
      deck" (now ✅ — it is implemented) and "Save to all subdecks" (now ⏳ —
      deferred convenience, not missing scheduling).

### US-2.19 — Deck sharing: subtree access model

**As** a student,
**I want** sharing a deck to cover its whole subtree (and not leak ancestors I
wasn't given),
**so that** what I can open/study matches what the teacher actually shared, and
my deck list is a consistent tree.

**Background / decisions (settled)**

Today `check_deck_visible` (the single access predicate behind `get_deck`,
`list_deck_classes`, `notes::list_notes`, and both study endpoints) checks only
the **single deck id** — owner → admin → collaborator → class-membership — with
**no recursion**. But the student *listing* and *counts* already recurse via
`WITH RECURSIVE subtree`. Result: a student shared a parent deck sees
**subtree-wide counts but only the parent row** — the children aren't listed, so
they can't be opened/studied individually. And a student shared only a *child*
deck has no visible parent, so their tree is incoherent.

Three decisions (documented divergence from Anki, where deck ownership is
single-user and trivially total):

1. **Grant = subtree.** Sharing deck X recursively grants X *and all its
   descendants* — for listing, counts, study, notes, everything that checks
   `check_deck_visible`.
2. **Ancestors are read-only context, not grants.** A student granted a *child*
   deck sees its ancestors in the list purely so the tree renders, but those
   ancestors are **not studyable** by them (only the granted node + its
   descendants are). We do **not** widen a child grant up the tree (that would
   silently expose the parent's own cards).
3. **The list is still a tree** for students — context-only ancestors render in
   place, marked non-studyable, never flattened away.

**Acceptance criteria**

- [x] `check_deck_visible` grants access when the deck **or any ancestor** is
      shared to the user (a student granted `Cell Biology` can study it and its
      descendants; a student granted `Biology 101` can study the whole subtree).
- [x] `list_decks` (student branch) returns, for a parent/child grant, the
      whole connected subtree — including **context-only ancestor rows** for a
      child grant — so the tree renders fully.
- [x] Study (`GET`/`POST /decks/{id}/study`) honours the subtree rule: studyable
      iff the deck or an ancestor is granted; a context-only ancestor returns a
      rejection (not its own cards).
- [x] Context-only ancestors are distinguished from studyable decks in the
      response (a `studyable` flag on `DeckResponse`, default true).
- [x] `GET /decks/counts` and card/note listing share the same subtree rule
      (counts expand to descendants of a grant).
- [x] Each criterion has a `server/tests/` test named after it
      (`server/tests/deck_sharing.rs`).

**Out of scope / related**

- Stage 7 personal `deck_options` overrides (scheduling tuning) — orthogonal to
  this access-model story.
- Teacher/admin sharing UX and collaborator nesting (reuse existing
  `share_deck`/`add_deck_to_class`; this story only changes how a grant
  *propagates* to descendants/ancestors).
- **Non-disclosure of admin meta for context-only ancestors** — `get_deck` of a
  context-only ancestor returns the deck row (for tree context) but empty
  `collaborators`/`classes`; a student not directly granted that deck never
  sees its collaborator emails or class roster (pinned by
  `deck_sharing::context_only_ancestor_detail_hides_collaborators_and_classes`).

Each item must keep the OpenAPI spec in sync (new/changed DTOs → regenerated
`/api-docs`).

---

### US-2.20 — Leech as per-student state (complete the leech action)

**As** a student who keeps failing a card,
**I want** the leech action to mark *my* card as a leech (and suspend it, when
configured) without affecting anyone else,
**so that** the leech signal is personal scheduling state, not shared-note
metadata.

**Background / decisions (settled)**

`apply_review` (`server/src/handlers/reviews.rs`) already detects leeches and
acts on them, but the marker is wrong: it writes `notes.leech_tagged_at` (a
**shared note** column) and `TagOnly` is a no-op. Since leech is **not** a tag
(see stage 6), the marker becomes per-student scheduling state.

Decisions recorded so implementation is unambiguous:

1. **`leech_at TIMESTAMPTZ` on `student_card_states`** (per `(student, card)`) is
   the single source of truth. Set when `lapses >= leech_threshold` on a review
   "Again". Null = not a leech. Replaces `notes.leech_tagged_at`.
2. **`TagOnly` = "mark leech, don't suspend"; `SuspendCard` = "mark + suspend".**
   Both write only that student's `student_card_states` row — no shared-note
   mutation, so a student's leech cannot leak to anyone else.
3. **The flag is per-student and survives a threshold change.** Lowering the
   threshold does **not** retroactively leech old cards (the marker is set at the
   moment of crossing, not recomputed from `lapses`).

**Acceptance criteria**

- [x] Migration: add `leech_at TIMESTAMPTZ` to `student_card_states` (new
      migration `0024_leech_per_student.sql`; never edit `0001`). Drop
      `notes.leech_tagged_at` (and its write in `apply_review`).
- [x] `apply_review` sets `leech_at = now` for *that* `(student, card)` when a
      review "Again" reaches the threshold; `TagOnly` marks without suspending,
      `SuspendCard` marks **and** sets `suspended = TRUE`.
- [x] Per-student isolation: student A leeching card X leaves student B's
      `student_card_states` row for X (`leech_at`, `suspended`) untouched, and
      the shared `notes` row is never mutated.
- [x] `StudyCard` (and `ReviewedCardState`) expose a `leech: bool` so the
      client can render the marker (`leech_at.is_some()`). Contracts + OpenAPI
      regenerated.
- [x] Each criterion has a `server/tests/` test named after it; `cargo test`
      passes and OpenAPI still generates.

**Out of scope / related**

- The client's leech "Tag Only" popup and any UI surfacing of the `leech` flag
  (already tracked as a deferred stage-4 client behaviour in
  [`docs/support/screens.md`](../support/screens.md)).
- `marked` (Anki's other system tag) — deferred to the future import/export
  stage (see stage 6).
- Undo of a leech (stage 8).

---

## 3. Preferences — server-side only   `[x]`

> **Scope:** most Anki preferences are client-side, form-factor-specific, or
> Anki-specific and do **not** belong in the server (see [`docs/support/preferences.md`](../support/preferences.md)
> "Scope philosophy"). Stage 3 implements only the preferences that change
> *shared scheduling behaviour* — `day_start_hour` (+ per-user `timezone`),
> `learn_ahead_seconds`, and `timebox_time_limit` (persistence-only) — plus the
> CRUD endpoint that edits them.

- [x] **Add preferences CRUD** — `GET/PATCH /preferences` (per-user), backed by
      the existing `user_preferences` table. Reads return the (possibly default)
      values; writes upsert. Covers the two persisted prefs now; add
      `timebox_time_limit` with the timeboxing story below. Added
      `preferences` handler + contract DTOs (`UserPreferences`/`UpdatePreferences`,
      minutes on the wire), routes/openapi wiring, and HTTP round-trip tests
      in `server/tests/preferences.rs`.
- [x] **Timezone correctness** — the day boundary is UTC-anchored via
      `day_start_hour`; add a per-user timezone so "next day starts at" means
      the user's local wall-clock time (see US-3.1 below). Implemented: per-
      user IANA `timezone` (default `UTC`), timezone-aware day boundary in
      `study.rs` via `chrono-tz`.
- [x] **Timebox time limit** — see US-3.2 below (persist the preference
      server-side; the timebox popup itself is a client-side concern, stage 4).
      Implemented: `user_preferences.timebox_time_limit` (minutes, default 0)
      + CRUD round-trip + bounds test.
- [x] Cross-check the remaining ⚪/❌ rows and record descope decisions in the
      matrix. **Done:** every preferences row is `✅` or `⚪` with a note — no
      outstanding `❌`/`🟡`. Client-side/form-factor rows point at stage 4;
      Anki-specific rows are marked "Decision: drop.".

### US-3.1 — Timezone correctness

**As** a student who does not live in UTC,
**I want** "Next day starts at" to mean local wall-clock time in *my* timezone,
**so that** my study day rolls over at the hour I set (e.g. 4 AM my time, not
4 AM UTC) and my daily limits, burial expiry, and "reviews today" counts match
my actual day.

**Background / problem being fixed**

`day_start_utc` in `server/src/handlers/study.rs` computes the study-day start by
applying `num_seconds_from_midnight()` to a `DateTime<Utc>`. That is,
`day_start_hour = 4` means **04:00 UTC for everyone**, regardless of where the
student lives. For a Sydney student (UTC+11) the boundary lands at 15:00 local —
daily limits reset mid-afternoon and "tomorrow's" cards become due ~9 hours
early. The preference value is *persisted*, but semantically wrong for anyone off
UTC.

**This is a per-user concern** (Anki is single-user, so it is a global setting
there; Anjuman is multi-user, so it must be per-user — same call made for the
rest of `user_preferences`).

**Acceptance criteria**

- [x] Add a per-user timezone to `user_preferences` and expose it on the wire:
      `UserPreferences`/`UpdatePreferences` gain a `timezone` field (IANA name,
      e.g. `"Europe/London"`; default `"UTC"`), round-tripped through
      `GET`/`PATCH /preferences` and reflected in the OpenAPI spec.
- [x] A helper resolves "start of study day" for a user by interpreting
      `day_start_hour` in the *user's* timezone and converting the result to a
      `DateTime<Utc>` (replaces the raw `day_start_utc`).
- [x] The study/scheduling path uses the timezone-aware boundary everywhere it
      currently calls `start_of_day` — day boundary for daily limits, burial
      auto-expiry, and study bucketing/`seen_today` remain consistent.
- [x] Daily-limit "today" (`effective_daily_limits`'s `NaiveDate`) is derived
      from the user's local day, so a limit reset happens at the user's local
      rollover, not UTC.
- [x] An invalid/unparseable timezone falls back to `UTC` (and a `0..=23`
      `day_start_hour` is still enforced).
- [x] Each criterion has a `server/tests/` test.

  **Implemented**: added `user_preferences.timezone` (migration `0020`), a
  `timezone` field on the preferences DTOs + handler (normalized to `UTC` on
  parse failure), a timezone-aware `day_start_local` replacing `day_start_utc`
  in `study.rs` (driven by `chrono-tz`), unit tests in `study.rs` and HTTP
  round-trip/fallback tests in `server/tests/preferences.rs`.

**Out of scope / decisions to document**

- **Scope decision**: timezone is *per-user*, not per-school and not per-preset
  (same decision as the rest of `user_preferences`; recorded in
  [`docs/support/preferences.md`](../support/preferences.md)). A future stage may revisit a school-wide default.
- **Storage format**: IANA name (`TEXT`), not a fixed `UTC+hh` offset — a fixed
  offset silently breaks across daylight-saving changes, whereas the IANA name
  follows the zone's own offset transitions.
- **Default**: `"UTC"` for now, preserving current behaviour exactly until a
  client/onboarding sets a real zone. Revisit later (school-based, location-
  based, etc.). **Decision recorded in [`docs/support/preferences.md`](../support/preferences.md).**
- **Analytics bucketing** (`analytics.rs`'s `start_of_today`/`start_of_week`,
  `DATE_TRUNC('day', …)`) is **not** in scope here — those are report roll-ups,
  not scheduling correctness; reconciling them to per-user local days is a
  separate story.
- **Anki parity note**: Anki is single-user and reads the OS timezone; there is
  no "per-user timezone" concept to mirror. This is an Anjuman-specific
  extension. The option is algorithm-neutral (neither SM-2 nor FSRS specific).

### US-3.2 — Timebox time limit (persist the preference; behaviour client-side)

**As** a student who studies in focused blocks,
**I want** my timebox interval ("show me a summary every N minutes") to be
saved in my preferences,
**so that** it follows me across devices and the client can prompt me at my
chosen cadence, without re-entering it on each device.

**Background / scope decision**

Timeboxing is a **client-side behaviour**. The Anki manual defines it as: *"If
you set the timebox time limit to a non-zero number of minutes, Anki will
periodically show you how many cards you've managed to study during the
prescribed time limit."* It is a study-session timer + popup — no server-side
scheduling effect at all (unlike `day_start_hour` and `learn_ahead_seconds`,
which do change due dates and limits).

So, per the established "server vs. client" split (mirroring US-2.14, the
selection-only deck options), **stage 3 does only the persistence half**: add the
column + wire it into the preferences contract + CRUD + OpenAPI, and test its
round-trip. The actual timebox popup is a stage-4 client story.

**Classification**: algorithm-neutral (neither SM-2 nor FSRS); no scheduling
implication.

**Acceptance criteria**

- [x] `user_preferences.timebox_time_limit` column added (minutes, `BIGINT`,
      default 0), `0` = disabled.
- [x] `UserPreferences`/`UpdatePreferences` gain `timebox_time_limit` (minutes
      on the wire, matching Anki's input), round-tripped through
      `GET`/`PATCH /preferences` and reflected in the OpenAPI spec.
- [x] Bounds enforced: `0..=9999` (Anki in-app range; `0` disables). Out-of-
      range → `400`.
- [x] A missing preference row still returns the default (`timebox_time_limit =
      0`), consistent with the other two prefs.
- [x] Each criterion has a `server/tests/` test.

  **Implemented**: migration `0021`, `timebox_time_limit` (minutes) on the
  preferences DTOs + handler with `0..=9999` bounds, and round-trip/default/
  bound tests in `server/tests/preferences.rs`.

**Out of scope / decisions to document**

- **Behaviour is client-side (stage 4)**: the periodic "N cards studied this
  timebox" prompt, its wording, and dismissal are a client concern. Anjuman's
  stateless, sessionless design makes this cleaner client-side — the client
  already knows the local answer count, so no server round-trip drives the
  timer.
- **No server-side enforcement**: the server stores and returns the value but
  never acts on it (no server notion of a live study session). Recorded so
  nobody later expects a backend timer.
- **No timebox history/metrics**: no persisted "reviews per timebox" ledger
  server-side; the client derives any running count locally from
  `ReviewResponse`/`StudyCounts`.
- **Scope**: `timebox_time_limit` is a *preference* (per-user), not a deck
  option — matching Anki (Preferences → Scheduler).

---

## 4. Frontend — implement the client   `[ ]`

Replace the counter-demo core with the real domain, consuming
`anjuman_contracts` and the server API. Follows the same discipline as stages
2–3: a story derives from [`docs/support/screens.md`](../support/screens.md) (the feature matrix), and
every acceptance criterion maps to a `CruxCore::update` test in `client/shared`
(see [`docs/process/PLANNING.md`](./PLANNING.md) §3 for the client test idiom — assert on `caps.effects()` and
injected `HttpResponse`s, never a running server).

> **Core vs. shell.** Acceptance criteria live in and test the **core**; the
> Leptos shell is thin glue and generally not unit-tested. Criteria are phrased
> as core behaviours, not UI prose.

- [x] **US-4.1 — Client plumbing (contracts + capabilities + test harness).**
      Front-load the API surface before any feature (see story below). Done:
      `anjuman_contracts` + `crux_http`/`url` wired into `shared`, `Http` effect
      variant, health-check flow proving the contract round-trip + two-event
      HTTP idiom, 7 passing tests.
- [x] **US-4.2 — Auth flow** (login → store JWT via `crux_kv` → `GET /me`).
      Everything downstream is authenticated, so this comes first (see story
      below). Done: contracts derives + auth core + 7 tests + Leptos
      `crux_http`/`crux_kv` capabilities + login screen.
- [ ] **Stable screens** — decks, notes, cards, study (one story per screen, in
      dependency order). Decks list is US-4.3 (done); study entry is US-4.4
      (done) and the study loop core is US-4.5 (done; Leptos rendering pending
      the shell agent); card styling (note-type CSS) is US-4.6 (done core+server;
      shell injection + dark-mode inversion pending); deck authorization
      predicate is US-4.7 (done) and teacher/admin study + real counts is
      US-4.8 (done). Cards browser is US-4.12; notes browser US-4.13; tagging UI
      US-4.14; classes/dashboard US-4.15; user admin US-4.16.
- [ ] **Card styling parity (Anki-faithful)** — extends US-4.6, which delivered
      the plumbing (note-type `styling` column + contracts + `StudyCard.styling`
      + `StudyCardView` + seeded default `.card` rule) and recorded the dark-mode
      inversion decision. Remaining fidelity per
      <https://docs.ankiweb.net/manual/templates/styling>: US-4.6.1 (`.card`
      wrappers so styling applies), US-4.6.2 (night-mode `.nightMode` rules), and
      US-4.6.3 (arbitrary CSS/HTML + platform classes + RTL pass-through).
- [ ] **Deferred client-side behaviours** — now in scope: on-screen timer + max
      answer seconds (US-4.17), audio/auto-advance/timebox/leech popup (US-4.18),
      preferences + theme/answer-keys (US-4.19). See [`docs/support/screens.md`](../support/screens.md).
- [ ] **Deck-options/preferences UI** — the settings screens that CRUD
      `deck-options` and `/preferences`. Deck-options: US-4.9 (view a preset,
      done), US-4.10 (edit + save a preset, done), and US-4.11 (assign a deck's
      preset, done) below. Preferences UI not yet drafted (and the US-4.9/US-4.10
      shell contracts — rendering the preset read-only + editable then saving —
      are pending the shell agent).
- [ ] Keep the FFI `Bridge`/`codegen` surface working as the model grows;
      regenerate bindings.

### US-4.1 — Client plumbing (contracts + capabilities + test harness)

**As** a developer building the real client,
**I want** the client core wired to `anjuman_contracts` and the HTTP/KV/time
capabilities with a working test harness,
**so that** subsequent feature stories have the shared wire types and the
`update`+`effects()` test idiom established up front.

**Background**

Today `client/shared` is the counter demo: a `Render`-only `Effect` enum, no
`anjuman_contracts` dependency, no HTTP/KV/time capabilities, and only the
counter tests. This story front-loads the API surface (per [`docs/process/PLANNING.md`](./PLANNING.md) §4) so
the client is pinned to the same `anjuman_contracts` DTOs the server serves.

**Acceptance criteria**

- [x] `anjuman_contracts` is a path dependency of `client/shared` (no `utoipa`
      `openapi` feature client-side).
- [x] `Effect` gains an `Http` (`crux_http`) variant; `crux_kv` and `crux_time`
      are added where the first features need them (KV for the stored JWT, time
      for learn-ahead/timers) — **deferred to US-4.2** (auth needs KV; not needed
      by this plumbing proof).
- [x] The core deserializes an `anjuman_contracts` DTO end-to-end in a test:
      a canned JSON body → a typed contract struct (proves the shared-type path).
- [x] A test demonstrates the two-event HTTP idiom: an event emits
      `Effect::Http(...)` with the right verb/URL, and a canned `HttpResponse`
      mutates `model` + requests `Render`.
- [x] `cargo test` passes in the client workspace; each criterion has a
      `shared` test named after it.

**Out of scope**

- Any real feature behaviour (auth, decks, study) — this story only proves the
  plumbing. Actual screens/features follow in their own stories.
- Non-Rust shell bindings (`boltffi`/`codegen`) — regenerated as the model
  grows in a later story, not blocked here.

### US-4.2 — Auth flow (login → store JWT → load `/me`)

**As** a user,
**I want** to log in with my email and password and have my session restored on
subsequent visits,
**so that** every authenticated action is backed by a stored JWT without me
re-entering credentials each time.

**Background**

The server exposes `POST /auth/login` (body `LoginRequest`, returns
`LoginResponse { token, user }`) and `GET /me` (Bearer-authenticated, returns
`UserResponse`). This story wires the core to both, stores the JWT via
`crux_kv` (which the shell persists to `localStorage`), and restores the
session on startup. It is the first *real* feature after US-4.1's plumbing, and
everything downstream (decks, study) depends on the stored token + the
`Authorization: Bearer <token>` header convention it establishes.

**Prerequisite (contracts)** — `LoginRequest`/`LoginResponse`/`UserResponse` are
server-oriented today (`LoginResponse`/`UserResponse` serialize but do **not**
deserialize; `LoginRequest` deserializes but does **not** serialize). The client
needs the mirror direction. Add the missing `Serialize`/`Deserialize` derives so
both ends round-trip the same DTOs.

**Acceptance criteria**

- [x] `Model` carries an auth state: `Unauthenticated` vs `Authenticated { token,
      user }` (or equivalent).
- [x] `Event::LoginSubmit { email, password }` emits an `Http` POST to
      `/auth/login` with a JSON `LoginRequest` body (assert method/URL/body).
- [x] A canned `LoginResponse` resolves to storing `token` + `user` in the model
      and emitting a KV `set` effect for the token (so the session survives
      reload).
- [x] A canned login **rejection** (401) leaves the model unauthenticated and
      surfaces the error (no token stored).
- [x] `Event::RestoreSession` reads the token from KV (`get`), and when present
      emits `GET /me` with `Authorization: Bearer <token>` (assert header).
- [x] `GET /me` success stores the `UserResponse`; `/me` 401 clears the stored
      token (session expired) and returns to `Unauthenticated`.
- [x] `Event::Logout` clears the token (a KV `delete` effect) and resets the
      model to `Unauthenticated`.
- [x] The contracts prerequisite is done: `LoginRequest: Serialize`,
      `LoginResponse`/`UserResponse: Deserialize` (server build + OpenAPI
      unchanged otherwise).
- [x] Each criterion has a `shared` test named after it; `cargo test` passes.

  **Implemented**: `contracts/auth.rs` derives; auth core (`Auth` state,
  `LoginSubmit`/`RestoreSession`/`Logout` + `LoginResult`/`MeResult`/`Token*`
  events, `pending_token` bridge) with 7 tests in `shared/src/app.rs`.

**Shell contract** (a checklist every shell satisfies — not a per-shell story; see
[`docs/client/CLIENT.md`](../client/CLIENT.md) §7 and [`docs/support/screens.md`](../support/screens.md) "Shell coverage").

- [x] Implement the `crux_http` capability (perform the fetch, feed bytes back
      via `resolve`) and `crux_kv` (localStorage) — replaces US-4.1's
      `Effect::Http(_)` stub.
- [x] Render a login form (email + password) forwarding `Event::LoginSubmit`;
      on startup, forward `Event::RestoreSession`.
- [x] Render `ViewModel` auth state (a signed-in vs. signed-out view), with a
      sign-out affordance forwarding `Event::Logout`.

> **Verified in Leptos now; SwiftUI/WinUI/Compose/Libadwaita check the same
> checklist off when they land.** Shell-specific gotchas (e.g. non-Rust shells use
> the FFI `Bridge` instead of the typed `Core`) are recorded inline as they arise.

**Out of scope**

- Registration, token refresh/rotation, role-based routing, and any protected
  screen beyond `/me` (decks/study are their own stories). Logout is **in
  scope** (clears the stored token and returns to `Unauthenticated`).

### US-4.3 — Decks list (read-only)

**As** a signed-in user,
**I want** to see my decks with their due counts,
**so that** I know what to study and can see at a glance how many new/learning/
review cards each deck has.

**Background**

`GET /decks` returns `Vec<DeckResponse>` (each deck with per-student counts when
listed); `GET /decks/counts` returns `DeckCountsResponse { decks: Vec<DeckCounts> }`.
Both are `AuthUser`-gated — any role may *view* decks (create/edit is
teacher/admin, deferred). This is the first stable screen and the template for
screens that follow (notes, cards, study): fetch a list over an authenticated
endpoint, deserialize `anjuman_contracts` DTOs, expose them in the `ViewModel`.

**Prerequisite (contracts)** — the deck response DTOs (`DeckResponse`,
`DeckCounts`, `DeckCountsResponse`) derive `Serialize` but **not** `Deserialize`
today. The client needs the mirror direction (same as US-4.2's auth fix). Add
`Deserialize` so the core can read the list/counts responses.

**Acceptance criteria**

- [x] `Model` gains a `decks` list (and an error flag) gated behind auth.
- [x] `Event::DecksRequested` (fired after a successful login/restore) emits
      `GET /decks` with the `Authorization: Bearer <token>` header.
- [x] A canned `Vec<DeckResponse>` (success) populates `model.decks` and the
      `ViewModel` exposes each deck's title + counts (`new`/`learning`/`review`).
- [x] `ViewModel.decks` is a **nested tree** (`DeckSummary.children`), built by
      the core from `parent_id`; orphans (`parent_id` pointing at a missing/
      absent deck) surface at the root in input order (amended).
- [x] A canned `DeckCountsResponse` (success) populates the per-deck counts used
      to render the due badges.
- [x] A rejection (401/500) sets an error in the `ViewModel` without crashing.
- [x] The contracts prerequisite is done: `DeckResponse`/`DeckCounts`/
      `DeckCountsResponse: Deserialize` (server build + OpenAPI unchanged).
- [x] Each criterion has a `shared` test named after it; `cargo test` passes.

  **Implemented**: `Deserialize` on the deck DTOs; `decks`/`decks_error` model
  state, `DecksRequested`/`DecksResult`/`DecksCountsResult` events (chained
  after login/`/me` success, cleared on logout), `ViewModel.decks` as a nested
  `Vec<DeckSummary>` (with `children`), tree built by the core via
  `build_deck_tree`; 7 deck tests (14 total in `shared`).

**Shell contract**

- [x] After sign-in, forward `Event::DecksRequested`. *(Auto-chained by the core — the shell just renders the result.)*
- [x] Render the decks list (title + due counts) from `ViewModel.decks`,
      **recursing into `DeckSummary.children`** (nested subdecks) — the core
      provides the tree; the shell only renders it.
- [x] Render an error state and an empty state (no decks yet).

> **Verified in Leptos now; other shells check the same list off later.**

**Out of scope**

- Create/rename/delete/duplicate/share decks (teacher/admin actions — a later
  story), deck *detail* (drilling into a deck), and study (its own story).

### US-4.4 — Open a deck (study entry)

**As** a signed-in user looking at my decks,
**I want** to select a deck and land on its study screen,
**so that** I can start studying that deck's cards.

**Background**

The decks list (US-4.3) already carries each deck's `id`. This story adds the
navigation seam: an `Event::OpenDeck { deck_id }` sets the *currently selected*
deck in the model and exposes it in the `ViewModel`, so the shell can switch
from the list to the study screen. No new fetch is required — the deck is
already in `model.decks` (the study *cards* are fetched by US-4.5).

**Acceptance criteria**

- [x] `Event::OpenDeck { deck_id }` records the selected deck
      (`model.selected_deck: Option<DeckSummary>`) and emits a render.
- [x] `ViewModel` exposes the selected deck (id + title + `studyable`) so the
      shell can show a study screen heading/context (and gate the study button).
- [x] Opening an unknown `deck_id` (not in the list) is a no-op that does not
      crash or select a bogus deck.
- [x] `Event::CloseDeck` returns to the deck list: clears `selected_deck` and
      resets in-flight study state (`current_card`/`counts`/`study_error`) while
      keeping auth + decks intact (carve-out for back-navigation).
- [x] Each criterion has a `shared` test named after it; `cargo test` passes.

  **Implemented**: `selected_deck` model state, `Event::OpenDeck`, `find_deck`
  selection helper, `ViewModel.selected_deck`; `DeckSummary` gained `studyable`
  (US-2.19 carve-out) so the shell can disable study on context-only decks.
  Later added `Event::CloseDeck` (back-navigation seam) for the shell's
  breadcrumb/sidebar. 4 new tests (18 total in `shared`). Logout clears selection.

**Shell contract**

- [ ] Tapping a deck forwards `Event::OpenDeck { deck_id }`.
- [ ] When `ViewModel` shows a selected deck, render the study screen (even if
      empty of cards until US-4.5); otherwise render the deck list.
- [ ] The breadcrumb/sidebar back-to-list affordance forwards `Event::CloseDeck`.

**Out of scope**

- Fetching/rendering study cards (US-4.5), and any per-deck detail beyond the
  study entry (collaborators/classes/rename — later stories).

### US-4.5 — Study session (single-card loop)

**As** a student in a deck,
**I want** to see one card at a time, answer Again/Hard/Good/Easy, and be shown
the next due card (or "nothing due"),
**so that** I can review the deck the way Anki does.

**Background**

The server exposes `GET /decks/{id}/study` (returns the first due card + counts)
and `POST /decks/{id}/study` (body `StudyAdvanceBody { card_id, rating,
response_time_ms }`, returns the next card + the reviewed card's new state). The
server is **sessionless**, so the *client* drives the loop: the `Model` holds a
single `Option<StudyCard>` + counts, and each answer POSTs and replaces the
current card with `next_card`. This mirrors the server's stateless contract and
avoids any client-side queue (which would drift from server state).

**Prerequisite (contracts)** — `StudyCard`/`StudyAdvance`/`StudyCounts`/
`ReviewedCardState` derive `Serialize` but **not** `Deserialize`, and
`StudyAdvanceBody` derives `Deserialize` but **not** `Serialize`. The client
needs the mirror direction (same as auth/decks).

**Acceptance criteria**

- [x] `Model` holds study-session state (`current_card: Option<StudyCard>`,
      `counts: StudyCounts`, plus the deck being studied).
- [x] `Event::StartStudy` emits `GET /decks/{id}/study` with the bearer header;
      the response's `next_card` becomes `model.current_card` (or `None` →
      "nothing due").
- [x] `Event::Answer { rating }` emits `POST /decks/{id}/study` with a
      `StudyAdvanceBody` carrying the current card id + rating; the response's
      `next_card` replaces `current_card`, and `counts` update.
- [x] Ratings are constrained to 1–4 (Again/Hard/Good/Easy); an invalid rating
      is ignored (no request emitted).
- [x] When `next_card` is `None`, the `ViewModel` exposes a "finished / nothing
      due" state (not a panic).
- [x] Study fetches signal in-flight state via `busy`: `StartStudy` sets it,
      `StudyStarted`/`StudyAdvanced` (Ok and Err) clear it — so the shell can
      distinguish "fetching" from "nothing due" (carve-out).
- [x] The contracts prerequisite is done: study DTOs round-trip
      (`Serialize`/`Deserialize` on the response types and `StudyAdvanceBody`).
- [x] **Deck-list counts stay in sync with study progress (write-through).**
      `StudyStarted(Ok)` and `StudyAdvanced(Ok)` fold `StudyAdvance.counts`
      (`new/learning/review/relearning`) back into the selected deck's entry in
      `model.decks`, so the deck-list badges and study-screen badges are one
      source of truth. `total_count` is left untouched (it's a card count, not a
      due count; `StudyCounts` doesn't carry it). No extra `/decks/counts`
      round-trip per answer.
- [x] Each criterion has a `shared` test named after it; `cargo test` passes.

**Count-sync decision (settled)**

Study progress updates the deck list **by write-through** from
`StudyAdvance.counts` (the authoritative per-deck count for the deck in hand),
not by re-fetching `/decks/counts` after each answer. Rationale: it's free (no
extra round-trip per card) and it's already the exact number the study screen
shows. The one field the study counts can't supply — `total_count` (a card
count, not a due count) — is preserved from the last `GET /decks`, which is
still correct because answer/bury/suspend don't change how many cards exist.

**Shell contract**

- [ ] When a deck is selected, forward `Event::StartStudy`.
- [ ] Render the current card (front; reveal back on demand), plus the four
      answer buttons (Again/Hard/Good/Easy) forwarding `Event::Answer { rating }`.
- [ ] Render the counts and the "nothing due" completion state, using
      `ViewModel.busy` to show "loading" while `busy && current_card == None`.

**Out of scope**

- Flip/reveal animation, and any offline/queueing. Predicted-interval labels
  are now in scope: `StudyCardView` exposes `predicted_interval` (rating→seconds)
  for the shell to format under each answer button (formatting itself is shell
  polish). Bury/suspend/reschedule/flag-from-study are US-4.5b.

---

### US-4.5b — Study actions (bury / suspend / reschedule / flag)

**As** a student mid-study,
**I want** to bury or suspend the current card (or its whole note), reschedule
it, or flag it,
**so that** I can act on a single card without leaving the study loop — matching
Anki's card/note action menus.

**Background**

The server already exposes all six actions (they were built for the card
browser, but are card/note-scoped and sessionless, so the study screen can drive
them with no server change):

- `POST /cards/{id}/bury` · `POST /cards/{id}/suspend` ·
  `PATCH /cards/{id}/reschedule` (`RescheduleBody { due_at?, days? }`)
- `POST /notes/{note_id}/bury` · `POST /notes/{note_id}/suspend`
- `PATCH /cards/{id}/flag` (`SetFlag { flag }`)

The client core was missing the *surface* to drive them from the study screen,
and `StudyCardView` was missing `note_id`/`flag` (both already on the wire
`StudyCard`). This story adds that surface only; the shell renders the buttons.

**Advance semantics (decision)**

- **Bury** and **suspend** (card *and* note) remove the card from the queue, so
  after a successful action the core re-fetches the next due card (chain
  `Event::StartStudy`), advancing past the affected card.
- **Reschedule** moves the card's due date into the future (out of the current
  queue), so it also advances via the same re-fetch.
- **Flag** does *not* remove the card — the core updates `current_card.flag` in
  place from the `FlagResponse` and re-renders only.

**Prerequisite (contracts)** — request/response bodies now round-trip:
`RescheduleBody` + `SetFlag` gained `Serialize`; `CardModResponse`,
`NoteModResponse`, `FlagResponse` gained `Deserialize` (they were wire-response
`Serialize`-only).

**Acceptance criteria**

- [x] `StudyCardView` carries `note_id` (note-level actions) and `flag`.
- [x] `Event::BuryCard`/`BuryNote`/`SuspendCard`/`SuspendNote` emit the matching
      `POST /cards/{id}/…` / `POST /notes/{note_id}/…` request with the bearer
      header, using the current card's id/note id.
- [x] `Event::RescheduleCard { days }` emits `PATCH /cards/{id}/reschedule` with
      a `RescheduleBody { days }` body; `Event::FlagCard { flag }` emits
      `PATCH /cards/{id}/flag` with `SetFlag { flag }`.
- [x] `flag` is constrained to 0–7; out-of-range (and actions with no current
      card) are ignored (no request emitted).
- [x] On success, bury/suspend/reschedule chain `StartStudy` (advance to the next
      card); `FlagDone(Ok)` updates `current_card.flag` in place.
- [x] Failures surface in `Model`/`ViewModel.card_action_error`.
- [x] Each criterion has a `shared` test named after it; `cargo test` passes.

**Shell contract**

- [ ] Render bury/suspend (card + note), reschedule, and flag affordances on the
      study screen, forwarding the events above. Reschedule likely prompts for a
      day offset; flag offers 0–7 (colors match Anki).
- [ ] Show `ViewModel.card_action_error` when an action fails.

**Out of scope**

- Undo/history, and the card-browser UI for these same actions (separate story).

**Deferred decisions (recorded for later)**

- **Reschedule input is `days` only.** The event is `RescheduleCard { days }`
  (Anki's "set due date" is a relative day offset). Anki also allows an
  **absolute date**; if we want parity (a date picker), that's a one-field event
  addition (`RescheduleCard { due_at }` or an extra `Option<DateTime>`) — the
  server already accepts both `due_at` and `days` via `RescheduleBody`, so no
  server change is needed. Defer until the shell builds the affordance and we
  decide whether a relative-offset spinner is enough or an absolute picker is
  wanted.
- **No undo.** Bury/suspend/reschedule are fire-and-forget with no undo hook.
  They are *reversible* server-side via the inverse endpoints
  (`POST /cards/{id}/unbury`, `POST /cards/{id}/unsuspend`, `POST
  /notes/{id}/unbury`, `POST /notes/{id}/unsuspend`), but there is no in-UI
  undo. This is intentional pending **stage 8 (History / Undo)**; when that
  lands, these actions should emit the same reversible-op records as every other
  mutation (per its `US-8.1` history-table design) rather than relying on
  carry-over from the browser UI.

---

### US-4.6 — Card styling (deliver template CSS + dark-mode inversion decision)

**As** a student reviewing a card,
**I want** the card to render with its note type's styling (not browser-default
HTML), and to respect dark mode,
**so that** cards look the way their author intended, day and night.

**Background / decisions (settled)**

Today `StudyCard` carries only the rendered `front`/`back` HTML — there is no
styling CSS anywhere in the model. In Anki a note type has three pieces: a
`front` template, a `back` template, and a **shared "Styling" CSS block** (see
<https://docs.ankiweb.net/templates/styling.html>), and Anki concatenates the
styling + rendered side into one self-contained card page.

Three decisions, recorded here so implementation is unambiguous:

1. **Styling is per note type, not per template.** The `styling` value lives on
   the `note_types` row (not `note_type_templates`) and is shared by every card
   of that note type. Field name **`styling`** everywhere (matching Anki's
   vocabulary), a plain `String` that mirrors `front`/`back`.
2. **Dark mode is Anki's inversion, not a re-theme.** Card authors write
   arbitrary CSS, so a palette/theme swap can't cover it. The shell injects a
   `night_mode` class + an inversion stylesheet into the card `<iframe>` when the
   app is dark (the opt-in `html.night_mode img { filter: invert(180deg) }`
   convention plus a coarse `filter: invert(...)`), exactly as Anki documents it.
   This is a **client-side** concern — the core/server only deliver `styling`; no
   server round-trip for dark mode.
3. **Default styling is Anki's default `.card` rule**, seeded onto existing note
   types so even untouched note types render sanely:
   ```css
   .card {
       font-family: arial;
       font-size: 20px;
       text-align: center;
       color: black;
       background-color: white;
   }
   ```

**Acceptance criteria**

- [x] Schema: `note_types` gains a `styling TEXT NOT NULL DEFAULT ''` column
      (new migration `0023_note_type_styling.sql`; never edit the applied
      `0001`/`0002`). `clone_note_type` copies `styling` to the copy.
- [x] Contracts: `NoteTypeResponse` and `UpdateNoteType` gain a `styling: String`
      / `styling: Option<String>` field (respectively); `StudyCard` gains
      `styling: String`. (No separate `CreateNoteType` body exists — note types
      are created via `clone_note_type`.)
- [x] Server `note_types.rs`: load `styling` in `get_note_type`; carry it on
      `NoteType`; expose it via `to_response`.
- [x] Server `note_types_handler.rs`: `update_note_type` persists `styling` when
      provided; `clone_note_type` copies the source `styling` (explicit column
      list — the DB default is *not* applied to an explicit INSERT).
- [x] Server `study.rs::row_to_study_card`: set `StudyCard.styling = nt.styling`
      (the note type is already fetched there).
- [x] Seed: set the default `.card` styling on both seed note types (new seed
      migration, appended to the same `0023` file).
- [x] `shared`: `StudyCardView` gains `styling: String`, populated from
      `StudyCard` in `From<&StudyCard>`.
- [x] Each criterion has a `server/tests/` test (or a `shared` test for the
      `StudyCardView` mapping) named after it; `cargo test` passes and OpenAPI
      still generates.

**Shell contract**

- [ ] Inject `StudyCardView.styling` into the card `<iframe>` (concatenated with
      the rendered side, Anki-style).
- [ ] Dark-mode inversion: this is tracked under `Theme
      (dark/light/follow-system)` in [`docs/support/screens.md`](../support/screens.md) (a ⚪ shell-only
      behaviour) — the shell applies the inversion class/stylesheet in the
      iframe, not the core/server. This story documents the *decision*; the shell
      work is a separate ⚪ cell, not a `shared` criterion.

**Out of scope** (for US-4.6 itself — superseded/covered by the styling-parity
stories immediately below)

- The shell's actual inversion implementation (the `Theme ⚪` cell), template
  editor UI for editing `styling`, and per-template CSS.

**Status — delivery done; fidelity (parity) pending.** US-4.6 delivered the
*plumbing* (the `styling` column, contracts, `StudyCard.styling`, the
`StudyCardView` mapping, the seeded default `.card` rule) and *documented* the
dark-mode decision. What remains is **Anki-faithful card styling behaviour**, per
<https://docs.ankiweb.net/manual/templates/styling> — captured by the three
stories below. The single most important gap: the renderer expands `{{Front}}`
into bare field text with **no** wrapping `.card` element, so the seeded
`.card { … }` rule is currently **dead CSS**. Anki resolves this by **injecting**
a `<div class="card">` (and `class="cardN"` for the Nth card type) *around* the
rendered template at render time — the template text stays bare (that is why the
Anki template editor shows only `{{Front}}`). See US-4.6.1.

#### US-4.6.1 — Inject the `.card` wrapper so note-type styling actually applies

**As** a student reviewing a card,
**I want** the card content wrapped in a `.card` element the way Anki does,
**so that** the note type's styling (`.card`, `.card1`, …) actually applies to
the card instead of being inert.

**Why:** Anki's renderer wraps each rendered card in `<div class="card">` (and
`<div class="cardN">` for the Nth card type of a multi-template note type),
**injected at render time**, not present in the template text — which is why the
template editor shows only `{{Front}}`. The wrapper is what makes both the
seeded `.card` rule and author card-scoped rules (`.card`, `.card1`, …) bind.
Without it, the seeded `.card { arial / 20px / centered / black-on-white }` rule
matches nothing.

**Acceptance criteria (core/server; the shell then consumes unchanged shapes)**

- [x] `note_types::render_card`/`render_cards` wrap the rendered front/back in a
      `.card` element, injected by the renderer (per Anki), so `StudyCard.front`
      contains `<div class="card">…</div>`. **The seed `front_pattern`/
      `back_pattern` stay bare** (`{{Front}}`, `{{Front}}<hr>{{Back}}`) — no seed
      migration changes template text.
- [x] The wrapper class reflects the card-type ordinal, matching Anki:
      ordinal 0 → `class="card"`, ordinal 1 → `class="card1"` (the
      `render_card`/`render_cards` callsites already pass `template_id`/the
      template, so the renderer needs the template's ordinal to emit the right
      suffix).
- [x] The renderer does **not** strip or rewrite author HTML *inside* the
      template — it still only expands `{{Field}}`; the `.card` wrapper is added
      *around* the expanded result, never into the author's own markup.
- [x] Tests named after the criteria:
      `render_card_wraps_sides_in_card_element`,
      `render_cards_suffixes_wrapper_class_by_ordinal`,
      `render_card_preserves_author_html_in_template` (all in
      `server/src/note_types.rs` unit tests) +
      `note_cards_wrap_front_and_back_in_card_wrapper` (`server/tests/notes_browser.rs`,
      the end-to-end "Basic" `.card` case). The multi-template `.card1` case is
      covered by `render_cards_suffixes_wrapper_class_by_ordinal`.
- [x] `cargo test` + OpenAPI generation pass; no new warnings.

**Dependency:** none — pure server renderer + no wire-shape change (the shell
keeps receiving `front`/`back`/`styling`; `front`/`back` now just carry the
`.card` wrapper).

**Divergence note (recorded):** Anki also applies the same `.card` wrapper in
its *browser preview*, and injects the wrapper per-card (one wrapper per card =
`front` and `back` are each individually wrapped, not the `.card` wrapping
the front+back pair as one element). We match: each of `front` and `back` is
wrapped independently in its own `.card`.

#### US-4.6.2 — Night-mode rules (`.nightMode` / `.card.nightMode`)

**As** a student reviewing in dark mode,
**I want** the card to honour Anki's `.nightMode` author CSS (in addition to the
coarse inversion already decided),
**so that** note types that define explicit night-mode colours actually use them.

**Why:** Anki's styling doc supports *both* (a) author-authored `.nightMode`
selectors in the styling block and (b) a client-side inversion fallback. US-4.6
documented (b); this story adds (a), i.e. the shell must *also* toggle a
`nightMode` class on the card root when dark, so `.card.nightMode { … }` and
`.nightMode .class { … }` rules bind.

**Acceptance criteria (shell/core boundary — no server change expected)**

- [ ] The shell injects a `nightMode` (or `night_mode`) class onto the card
      `<body>`/root of the `<iframe>` `srcdoc` when the app is dark, alongside the
      existing inversion stylesheet.
- [ ] The `shared`/`StudyCardView` surface is unchanged; this is shell-side class
      toggling (documented as such, mirroring the `Theme ⚪` cell per
      [`docs/support/screens.md`](../support/screens.md)).
- [ ] Any `server/tests/`/`shared` test is **not** required (client-only class
      toggle); instead a `docs/client/SHELLS.md` (or `docs/support/screens.md`)
      note records the `.nightMode` convention and that author CSS wins over the
      inversion where both apply.

#### US-4.6.3 — Arbitrary CSS/HTML, platform classes, and RTL pass-through

**As** a note-type author,
**I want** my arbitrary CSS/HTML (field-level `<div class=…>` styling,
platform-specific selectors like `.android`/`.mobile`/`.iphone`, `direction: rtl`,
image-resizing overrides, `@font-face`) to render faithfully,
**so that** cards behave like they do in Anki rather than being stripped or
re-themed.

**Why:** most of the Anki styling manual is *already* satisfiable once (a) the
`.card` wrapper exists (US-4.6.1) and (b) the styling CSS is injected verbatim
(US-4.6). This story is the **fidelity gate** — it verifies we pass through
arbitrary CSS/HTML unmodified and decides which platform/RTL classes are
in-scope, rather than re-implementing them.

**Acceptance criteria / decisions (record — explicit deferrals allowed)**

- [ ] Confirm the renderer + shell do not sanitize or rewrite author CSS/HTML
      (arbitrary tags/classes/`@font-face`/`<script>` pass through unmodified).
      Record the decision in `docs/support/screens.md`.
- [ ] Platform-specific classes (`.win`/`.mac`/`.linux`/`.android`/`.mobile`/
      `.iphone`/`.ipad`): decide whether the shell sets a platform class on the
      card root (recommended for the current single Leptos shell: `.linux`) or
      defers to when multiple shells exist. **Deferred** — document, do not
      implement now.
- [ ] RTL `direction: rtl` and image-resizing (`img { max-width: none }`): these
      are pure CSS and work once pass-through is confirmed — capture as a
      regression note, not new code.
- [ ] Audio replay-button styling (`.replay-button`) is out of scope until audio
      is implemented (US-4.18); the class name is reserved in the doc only.

---

### US-4.7 — Centralize deck-access authorization behind a permission predicate

**As** a developer adding or changing a deck feature,
**I want** a single source of truth that answers "what may this user do to this
deck?",
**so that** authorization is computed in one place and later permission/scope
changes are one edit, not a sweep across handlers.

**Background / decisions (settled)**

Deck authorization is currently spread across three ad-hoc mechanisms, which is
what produced the inconsistencies US-4.8 fixes:

1. **Role checks** — local `check_teacher_or_admin` copies in `decks.rs`,
   `classes.rs`, `deck_options_handler.rs`, plus inline `claims.role == X`
   branches.
2. **Ownership/collaboration checks** — `check_deck_owner`,
   `check_deck_collaborator`, `is_deck_collaborator`.
3. **Resource-grant checks** — `has_grant` / `deck_access` / `check_deck_visible`
   / `check_deck_studyable` (the US-2.19 subtree model).

This story consolidates **deck** authorization into one predicate. Two decisions,
recorded here because they are the whole point of the change:

1. **Permissions are resource-grants, not role-gates.** "May study deck X" is a
   function of the user's *relationship* to X (owner / collaborator / class
   member, extended over the subtree per US-2.19), **not** of their role label
   alone. Roles only influence which grants a user tends to hold (teachers own
   decks; admins can manage any deck). This is what makes the model stable when
   roles or scoping are later re-tuned.
2. **Admins are NOT blanket-granted study on every deck.** The existing
   `has_grant` admin short-circuit (`Admin → true`) is removed; an admin holds a
   grant the same way anyone does (owner/collaborator/class). Management
   permissions (create/rename/delete/share) stay admin-permissive as today — this
   story only rationalises the **study/access** predicate.

**Shape (to implement)**

- A single module (e.g. `server/src/permissions.rs`) exposing a deck permission
  enum/predicate, e.g. `DeckPerm { Study, Manage, Share, ReadAdminDetail, … }`,
  and one entry point like `deck_permission(db, claims, deck_id) -> DeckPerm` (or
  `require_deck_perm(…, DeckPerm::Study)` for the deny case).
- The existing `has_grant` / `deck_access` / `check_deck_visible` /
  `check_deck_studyable` become the *implementation* behind that predicate; new
  call sites (and the study/counts call sites US-4.8 touches) call the predicate,
  not `role ==` directly.

**Acceptance criteria**

- [x] The deck-access predicate exists and answers `Study` for a deck the user
      owns, collaborates on, or reaches via a class (extended over the subtree
      per US-2.19) — and `Study` is **not** granted merely for being `Admin`.
- [x] `check_deck_studyable` (and study `GET`/`POST`) consult the predicate; an
      admin who does not own/collaborate a deck gets `403` (behaviour change from
      today's blanket grant).
- [x] `has_grant` no longer short-circuits `Admin → true`.
- [x] The predicate is `pub` and reusable from `list_decks`/`deck_counts` so
      US-4.8 can compute `studyable` from it (no duplicate logic).
- [x] Each criterion has a `server/tests/` test named after it (e.g.
      `server/tests/deck_permissions.rs`); `cargo test` passes and OpenAPI still
      generates.

**Non-goals (do this later, not here)**

- A generic role→permission table / dynamic RBAC engine. This is a *coded*
  domain predicate for decks, not a configurable permission store.
- Migrating every `check_teacher_or_admin` across the server (classes, deck
  options, analytics, dashboard, card/note editing). Those are a separate,
  follow-on sweep; only deck *access/study* is in scope here.

**Shell contract**

- [ ] None — no wire/`ViewModel` change.

---

### US-4.8 — Teachers/admins can study decks, with real per-state counts

**As** a teacher (or admin) who owns/collaborates on a deck,
**I want** to open and study that deck exactly like a student — including real
per-state due counts on the deck list,
**so that** I can preview my own material and its scheduling the way my students
will see it.

**Prerequisite:** US-4.7 (deck-access permission predicate) — this story consumes
its `Study` permission for `studyable`/`check_deck_studyable` rather than
re-hardcoding role checks.

**Background / decisions (settled)**

Study already *works* for teachers/admins: `deck_advance` calls
`ensure_card_states_for_deck(claims.sub, …)`, so a teacher/admin studying a deck
lazily gets their **own** `student_card_states` rows — they study through the
same per-user scheduling path as a student.

But the deck *list* and *counts* are inconsistent with that:

1. `list_decks` hardcodes `studyable: true` for the admin and teacher branches
   (rather than computing it via the US-4.7 predicate).
2. `list_decks` (admin + teacher branches) and `GET /decks/counts` (teacher/
   admin branch) return **no per-state counts** — `new_count`/`learning_count`/
   `review_count`/`relearning_count` are `None`/hardcoded `0`, and only
   `total_count` is populated. The core maps `None` → `0`, so admin/teacher deck
   lists show `0` for new/learning/review.

One decision, recorded so this is unambiguous:

- **Teachers/admins may study the decks they hold a grant on** (owned or
  collaborated), using their **own** per-user scheduling state — the same code
  path students use. This is now *by design*, matching Anki's owner-preview
  reality. (Whether an *admin's* grant is blanket or owner/collaborator-based is
  settled in US-4.7.)

**Acceptance criteria**

- [x] `list_decks` admin + teacher branches compute `studyable` from the new
      deck-access permission predicate (US-4.7) — i.e. `Study` is granted —
      instead of hardcoding `true`, and are scoped to the decks the caller
      actually holds a grant on (not every school deck).
- [x] `list_decks` admin + teacher branches return real per-state counts by
      calling `deck_counts_for_student(claims.sub, deck_id)` (and real
      `total_count`), so new/learning/review/relearning are populated —
      mirroring the student branch.
- [x] `GET /decks/counts` teacher/admin branch returns per-state counts (via the
      same `deck_counts_for_student`) instead of hardcoded `0`.
- [x] `studyable` and counts are consistent: a deck marked studyable in the list
      is one the `Study` permission (US-4.7) admits, and its counts match what
      the study flow would show.
- [x] Each criterion has a `server/tests/` test named after it (e.g. in a new
      `server/tests/teacher_admin_study.rs`); `cargo test` passes and OpenAPI
      still generates.

**Shell contract**

- [ ] No new core/`ViewModel` fields — `DeckSummary.studyable` and the count
      fields already flow through. The shell should continue to gate the study
      affordance on `studyable` (now correct for teachers/admins).

**Out of scope**

- Sharing/owner UX for adding a collaborator (existing `share_deck`/
  `add_deck_to_collaborators`), and any role that can study *without* a grant.

---

### US-4.9 — Deck-options settings screen (view a preset for the selected deck)

**As** a signed-in teacher/admin viewing a deck,
**I want** to see that deck's scheduling preset (its deck options values) read
straight from its `options_id`,
**so that** I can understand how a deck is currently configured before editing it.

**Background**

The deck-options backend is **already complete** (stage 2): the server exposes
`GET /deck-options` (list presets), `GET /deck-options/{id}` (one preset), and
`POST`/`PATCH`/`DELETE /deck-options`. The `DeckOptions` response DTO already
derives **both** `Serialize` and `Deserialize` (`contracts/src/deck_options.rs`),
so unlike US-4.2/4.3/4.5 there is **no contracts `Deserialize` prerequisite** for
reading a preset. This story is the **read-only half** of the settings screen: it
resolves *which* preset the selected deck uses and renders its values.

Two findings, recorded so implementation is unambiguous:

1. **`DeckResponse` does not expose `options_id`.** A deck's preset link is not
   on the wire today, so the core cannot resolve "which preset does this deck
   use" from the already-fetched deck. This story needs a **carve-out** (core
   agent): add `options_id` to `DeckResponse` and populate it in
   `list_decks`/`get_deck` (mirroring how US-2.19/US-4.4 added `studyable`).
2. **`options_id` is the *effective* id, resolved server-side — never null on
   the wire.** A deck with `decks.options_id = NULL` falls back to the global
   default preset (system school, id 0). The server resolves this when building
   the response, so `DeckResponse.options_id: i64` is always a concrete, **live**
   preset id (`decks.options_id.unwrap_or(0)`). The client never applies the
   fallback itself, and — because id 0 is the system default (never deleted) and
   `ON DELETE SET NULL` turns a deleted preset back into `NULL` → 0 — the client
   can never fetch a dangling id, so no 404-on-deleted-preset handling is needed.

**Prerequisite (contracts)** — `DeckResponse` gains
`options_id: i64` (`Serialize` + `Deserialize`), populated server-side as the
*effective* preset id (`decks.options_id.unwrap_or(0)`). This is the one
interface change before client work.

**Acceptance criteria**

- [x] `DeckResponse` carries `options_id: i64` (`Serialize` + `Deserialize`);
      the server populates it in `list_decks`/`get_deck` as the **effective** id
      (`decks.options_id.unwrap_or(0)`), so a deck with no preset reports `0`
      (no behaviour change otherwise; OpenAPI still generates).
- [x] `Model` gains deck-options state for the selected deck: the resolved
      preset id + an `Option<DeckOptions>` (and an error flag), cleared on
      `CloseDeck`/logout.
- [x] `Event::DeckOptionsRequested { deck_id }` (fired from the selected deck's
      context) emits `GET /deck-options/{options_id}` with the bearer header,
      where `{options_id}` is the deck's effective `options_id` (no client-side
      fallback).
- [x] A canned `DeckOptions` response (success) populates the model and exposes
      the preset's values in the `ViewModel` (name + the field inventory from
      `docs/support/deck-options.md`, including enums serialized as their wire
      forms).
- [x] A deck with no assigned preset (server reports `options_id: 0`) resolves
      to the global default preset — the core requests `GET /deck-options/0` and
      renders it, rather than treating it as absent.
- [x] A rejection (403/500) sets an error in the `ViewModel` without crashing.
- [x] Each criterion has a `shared` test named after it; `cargo test` passes.

**Shell contract**

- [x] From a deck's context, forward `Event::DeckOptionsRequested { deck_id }`.
- [x] Render the preset's values read-only (name + each option, grouped as the
      matrix groups them: daily limits / new cards / lapses / display order /
      burying / audio / timers / auto-advance / FSRS / advanced).
- [x] Enums render as human labels (see the wire `snake_case` values), not raw
      debug strings.
- [x] Render an error/empty state.

**Out of scope**

- Editing/saving the preset (US-4.10), preset management (create/clone/rename/
  delete), per-deck `preset`/`this_deck`/`today_only` override UI, and the
  selection-only *behaviours* (timer/audio/auto-advance) — those are separate
  stage-4 stories already queued.

---

### US-4.10 — Deck-options settings screen (edit + save a preset)

**As** a signed-in teacher/admin viewing a deck's preset,
**I want** to edit its deck-options values and save them back to the server,
**so that** I can tune a deck's scheduling (limits, steps, leeches, ordering, …)
without leaving the app.

**Background**

Builds on US-4.9 (which resolves & renders the preset read-only). The server
already supports `PATCH /deck-options/{id}` with an `UpdateDeckOptions` body
(every field optional). `UpdateDeckOptions` derives **`Deserialize` only** — it
is a *request* body, so the client must **serialize** it. This is the mirror of
US-4.5's `StudyAdvanceBody` prerequisite.

**Prerequisite (contracts)** — `UpdateDeckOptions` gains `Serialize` (and
`CreateDeckOptions` too, if we want a "new preset" path here — otherwise defer
create to a preset-management story). Server build + OpenAPI unchanged otherwise.

**Validation** — the client must mirror the matrix's min/max/defaults, since the
server rejects out-of-range values (and we don't want a save to fail on something
the UI should have caught):

- `new_per_day` / `review_per_day` / `leech_threshold`: 0..=9999 (leech 1..=9999)
- `desired_retention`: 0.70..=0.99
- `maximum_answer_seconds`: 1..=7200; `maximum_interval`: 1..=36500
- `auto_advance_*` seconds: 0.0..=9999.0 (1 dp); `easy_days`: exactly 7 items
- step strings (`learning_steps`/`relearning_steps`): Anki-style `"1m 10m"`

**Acceptance criteria**

- [x] The contracts prerequisite is done: `UpdateDeckOptions: Serialize`
      (server build + OpenAPI unchanged).
- [x] `Event::DeckOptionsSave { id, update }` (or a per-field edit → a single
      save) emits `PATCH /deck-options/{id}` with a JSON `UpdateDeckOptions` body
      carrying only the changed fields (assert method/URL/body).
- [x] A canned `DeckOptions` response (success) updates the model's preset and
      re-renders; a 4xx/5xx surfaces a save error without losing the edited
      values (so the user can correct and retry).
- [x] Local validation rejects out-of-range values (per the matrix ranges above)
      *before* emitting the PATCH — no request is emitted for an invalid value.
- [x] Each criterion has a `shared` test named after it; `cargo test` passes.

**Shell contract**

- [ ] Render the preset as **editable** controls (text/number inputs, booleans,
      enums as selectors, step strings as text), one per matrix option.
- [ ] Apply the client-side min/max clamping/messages before forwarding the save.
- [ ] Forward `Event::DeckOptionsSave { .. }` on save; show a saving/error/
      saved state.

**Out of scope**

- Preset *management* (create/clone/rename/delete/deassign), the per-deck
  `preset`/`this_deck`/`today_only` override UI, and the selection-only
  behaviours (timer/audio/auto-advance). The FSRS parameter *block* (the raw
  weight vector `fsrs_parameters`) is displayed in US-4.9 and left read-only
  here — the optimizer is deferred (US-2.18b).

---

### US-4.11 — Assign a deck's preset (change the deck → preset link)

**As** a signed-in teacher/admin viewing a deck,
**I want** to change which scheduling preset the deck uses (or clear it back to
  the default),
**so that** I can point a deck at a different set of options without recreating
it.

**Background / decisions (settled)**

The capability exists today only as a side-effect of `PATCH /decks/{id}/rename`
(`UpdateDeck.options_id`), which is both mis-named ("rename") and subtly broken:
its validation rejects assigning the global default preset (id 0) to a deck from
a real school, because it compares against `claims.school_id` and preset 0 is
owned by the system school — the same cross-school bug US-4.9's carve-out fixed
for the read path. Assignment is also invisible to the current client (US-4.9/
4.10 only read/edit the preset a deck *already* has).

This story adds a first-class, correctly-named assignment operation and makes its
id-0 semantics consistent with the read path (US-4.9) and the write path
(US-4.10). Three decisions recorded so implementation is unambiguous:

1. **A dedicated endpoint, not `rename`.** Add `PATCH /decks/{id}/options` with
   a small body, `SetDeckOptions { options_id: i64 }`, and **remove** the
   `options_id` field from `UpdateDeck`/`rename_deck` (so there's a single,
   clearly-named way to assign a preset).
2. **`options_id: 0` means "use the default" — no null.** The body is a plain
   non-nullable `i64`, symmetric with `DeckResponse.options_id` (so a client can
   round-trip the id it read back into a request). `0` maps to
   `decks.options_id = NULL` on write (the DB's "unassigned" state, which the
   read path already resolves back to the effective id `0`); `n != 0` sets
   `decks.options_id = n`. Storing `NULL` (not literal `0`) keeps the
   cross-school-scoped system preset row out of real-school decks' FKs.
3. **Assignment validation is school-scoped for real presets.** `n != 0` must
   exist **and** belong to the caller's school (`opt.school_id ==
   claims.school_id`); otherwise 400. This matches US-4.10 (a user's own presets
   are editable; the system default is read-only) and keeps other schools'
   presets unassignable.

**Acceptance criteria**

- [x] `PATCH /decks/{id}/options` exists (new route + handler), accepting
      `SetDeckOptions { options_id: i64 }`; `UpdateDeck` no longer carries
      `options_id` (OpenAPI regenerated; no route for it remains).
- [x] Assigning a preset in the caller's school (`options_id: n`, `n != 0`)
      updates `decks.options_id` and the response reflects the new effective id `n`.
- [x] `options_id: 0` clears the assignment (`decks.options_id = NULL`); the
      effective id then resolves to `0` (the read path already maps NULL → 0).
- [x] Assigning a preset from a different school (or a nonexistent id) is
      rejected (400) — no cross-school assignment.
- [x] Each criterion has a `server/tests/` test named after it; `cargo test`
      passes and OpenAPI still generates.

**Shell contract**

- [ ] In the deck-options screen, add an "Assign preset" affordance that lists
      the school's reusable presets (`GET /deck-options`) plus a "Default (id 0)"
      option, forwarding `Event::DeckOptionsAssign { deck_id, options_id }`.
- [ ] When the loaded preset is the global default (id 0), the edit/save
      affordance stays read-only; assigning one of the school's own presets
      enables editing.

---

### US-4.12 — Card browser (list cards, filter/sort/page, per-card actions)

**As** a signed-in user browsing a deck,
**I want** to list the cards in a deck (filterable by note type / state / flag,
sortable, paginated) and act on an individual card,
**so that** I can find and manage cards without going through the study loop.

**Background**

The server already exposes `GET /cards` (`CardBrowserQuery` filters: `deck_id`,
`note_type_id`, `state`, `flag`, `q`, `sort`, `page`, `per_page`) returning a
`CardBrowserPage { cards, page, per_page, total }`, and the per-card action
endpoints built for US-4.5b (`suspend`/`unsuspend`/`bury`/`unbury`/`reschedule`/
`flag`). The client has **no** browser surface: no events, no `Model`/`ViewModel`
fields, no tests.

**Prerequisite (contracts)** — `CardBrowserResponse`/`CardBrowserPage` derive
`Serialize` only (wire *responses*); they need `Deserialize` for the client.
`CardBrowserQuery` derives `Deserialize` only (wire *request*); it needs
`Serialize` to build query params.

**Acceptance criteria**

- [x] `Event::CardsRequested { filter: CardBrowserFilter }` emits `GET /cards`
      with the folded query params (deck/note-type/state/flag/q/sort/page/
      per-page, CSV multi-values via `to_wire_query`) + bearer header.
- [x] `CardsResult(Ok)` populates `Model.browser_cards`/`browser_page`/
      `browser_per_page`/`browser_total`, surfaced via `CardBrowserView` (Facet
      mirror); `Err` sets `browser_error`.
- [x] A selected browser card (`BrowserSelectCard`) supports
      `BrowserSuspendCard`/`BrowserBuryCard`/`BrowserRescheduleCard`/
      `BrowserFlagCard` (by card id, independent of `Model.current_card`);
      completions refresh the list via the last stored filter (`refresh_browser`),
      and `BrowserFlagDone` updates the card's flag in place.
- [x] Each criterion has a `shared` test named after it; `cargo test` passes (77).

**Shell contract**

- [x] Render the card table (front/back preview, deck, state, due, flag,
      suspended/buried), the filter/sort controls (incl. the note-type dropdown
      fed by `NoteTypesRequested`), and pagination.
- [x] Per-row action menu (suspend/bury/reschedule/flag) forwarding the events.
- [x] (carve-out) note-type list (`NoteTypesRequested`/`vm.note_types`) added to
      the core; shell consumed it for the filter dropdown.

**Out of scope**

- Card *edit* (fields/templates) — that's the notes browser (US-4.13); move-to-
  another-deck is a follow-up.

---

### US-4.13 — Notes browser (list/edit notes, note-type CRUD)

**As** a signed-in teacher/admin,
**I want** to browse and edit notes (change fields, note type, delete) and manage
note types + templates,
**so that** I can author content the way Anki's browser does.

**Background**

The server exposes `GET /notes` (list, optional `deck_id` filter),
`POST/PATCH/DELETE /notes/{id}` (create/update/delete with `CreateNote`/
`UpdateNote`), and the note-type/template endpoints
(`GET/PATCH/DELETE /note-types`, `…/templates`). The client has **no** notes
model (this is why US-6.5b per-note tag CRUD was deferred). Note scoping:
teacher/admin only (gate via `vm.permissions.can_manage_notes` in the shell).

**Prerequisite (contracts)** — `NoteResponse` + `CardSummary` gain `Deserialize`;
`CreateNote`/`UpdateNote`/`ListNotesQuery` gain `Serialize`.

**Acceptance criteria**

- [x] `Event::NotesRequested { deck_id }` emits `GET /notes` (+ bearer, optional
      `deck_id` query); `NotesResult(Ok)` populates `Model.notes` → `NoteView`
      (Facet mirror: fields flattened to strings, tags to names); `Err` sets
      `notes_error`.
- [x] `Event::NoteSave`/`NoteCreate`/`NoteDelete` emit `PATCH`/`POST`/`DELETE
      /notes/{id}` with the appropriate body (string fields folded via
      `fields_to_map`); completions refresh the list via the stored filter.
- [x] `NoteTypesRequested` already delivered (US-4.12 carve-out); reused here.
- [x] Each criterion has a `shared` test named after it; `cargo test` passes (83).

**Shell contract**

- [ ] Render the notes table + editor (fields keyed by note-type field names),
      gate on `vm.permissions.can_manage_notes`, and forward the events.

**Out of scope**

- Note-type/template *editing UI* (front/back pattern + styling) is a follow-up
  if not already server-covered; card generation preview; per-note tag CRUD
  (US-4.14).

---

### US-4.14 — Tagging UI (sidebar tree + per-note tag CRUD)

**As** a signed-in teacher/admin viewing notes,
**I want** to see the tag tree and add/remove tags on a note,
**so that** I can organise content by taxonomy.

**Background**

The core already ships the distinct tag list + `::` tree (`ViewModel.tags` /
`tag_tree`, US-6.5a). What's missing is **per-note** tag CRUD — deferred as
US-6.5b because there was no notes browser to hang it on. Now that US-4.13
provides notes, this story adds the note-tag actions.

**Acceptance criteria**

- [ ] `Event::AddTags { note_id, tags }` emits `POST /notes/{id}/tags` with
      `AddTags`; `Event::RemoveTags { note_id, tags }` emits `DELETE
      /notes/{id}/tags` with `RemoveTags`.
- [ ] Results refresh the note's `tags` (re-fetch or merge) and re-render.
- [ ] Each criterion has a `shared` test named after it.

**Shell contract**

- [ ] Render the tag tree in the sidebar (already in `ViewModel.tag_tree`), and
      per-note tag add/remove controls gate on `vm.permissions.can_manage_tags`.

---

### US-4.15 — Classes / roster / dashboard (teacher + admin)

**As** a signed-in teacher/admin,
**I want** to manage classes (create/rename/archive/delete, add/remove members)
and view the dashboard + analytics,
**so that** I can run my school's groups and see their progress.

**Background**

The server exposes the full `/classes` surface, plus `/dashboard` and
`/analytics`. The client has nothing. Gate on `vm.permissions.can_manage_classes`
/ `can_view_dashboard` / `can_view_analytics`.

**Acceptance criteria**

- [ ] `Event::ClassesRequested` emits `GET /classes`; results populate a
      `ClassSummary` list.
- [ ] `Event::ClassCreate`/`ClassRename`/`ClassDelete`/`ClassArchive` emit the
      matching `/classes`/`/classes/{id}` requests (admin vs. teacher shaping
      per role, matching the server).
- [ ] `Event::DashboardRequested` / `AnalyticsRequested` emit `GET /dashboard` /
      `GET /analytics/me`; results populate their views.
- [ ] Each criterion has a `shared` test named after it.

**Shell contract**

- [ ] Render the classes list/roster + dashboard, gated on the permissions
      object; forward the events.

---

### US-4.16 — User administration (admin only)

**As** a signed-in admin,
**I want** to list, view, create, update, and delete users in my school,
**so that** I can onboard and manage accounts.

**Background**

The server exposes `/users` (`GET` list, `POST` create, `GET/PATCH/DELETE
/users/{id}`). Gate on `vm.permissions.can_manage_users` /
`can_administer_school`.

**Acceptance criteria**

- [ ] `Event::UsersRequested` emits `GET /users`; `UsersResult` populates a
      `UserSummary` list.
- [ ] `Event::UserCreate`/`UserUpdate`/`UserDelete` emit the matching
      `/users`/`/users/{id}` requests; results refresh the list.
- [ ] Each criterion has a `shared` test named after it.

**Shell contract**

- [ ] Render the admin users screen, gated on `can_manage_users`; forward the
      events.

---

### US-4.17 — Client-side behaviours I (on-screen timer, max answer seconds)

**As** a student mid-study,
**I want** an on-screen timer that shows my per-card time (and clamps it to
`maximum_answer_seconds`), and for that time to be sent as `response_time_ms`,
**so that** I can see my pace and the analytics get accurate timing.

**Background**

Server already persists `show_on_screen_timer`, `stop_timer_on_answer`,
`maximum_answer_seconds`; the client must implement the behaviour (see
`screens.md` "deferred behaviours"). These are pure shell behaviours — no new
core surface, but each still needs a story + (where testable) a `shared` test.

**Acceptance criteria**

- [ ] `Answer { response_time_ms }` is extended so the core sends the elapsed
      ms (add the field to the `Answer` event; already `Option<i64>` on the
      wire body).
- [ ] The shell runs the timer, stops on answer reveal per `stop_timer_on_answer`,
      clamps to `maximum_answer_seconds`.

---

### US-4.18 — Client-side behaviours II (audio, auto-advance, timebox, leech popup)

**As** a student mid-study,
**I want** audio playback honours the audio prefs, auto-advance fires per the
config, a timebox popup appears per `timebox_time_limit`, and leech cards show a
popup,
**so that** the study experience matches the persisted preferences.

**Background**

Server persists `dont_play_audio_automatically`, `skip_question_when_replaying_
answer`, `auto_advance_*`, `timebox_time_limit`, `leech_action`. Pure shell
behaviours (see `screens.md`). Any that need a core signal (e.g. exposing the
auto-advance action enum to the shell) get a small core surface + test.

**Acceptance criteria**

- [ ] Any newly-required core surface (e.g. the resolved `auto_advance_*` action
      as a shell-facing enum) is added with a `shared` test.
- [ ] The shell implements the behaviours listed, respecting the persisted flags.

---

### US-4.19 — Preferences UI + form-factor settings (theme, answer keys)

**As** a signed-in user,
**I want** to view and edit my preferences and form-factor settings (theme,
answer keys),
**so that** I can personalise the app.

**Background**

`/preferences` (get/patch) is server-persisted; the client has no settings screen
(US-3 stage was server-only). Theme is client-owned (follow-system/dark/light).
Answer-key bindings are form-factor specific (shell-owned).

**Acceptance criteria**

- [ ] `Event::PreferencesRequested` emits `GET /preferences`; results populate a
      `PreferencesView`.
- [ ] `Event::PreferencesSave` emits `PATCH /preferences` with the changed fields.
- [ ] Each criterion has a `shared` test named after it.

**Shell contract**

- [ ] Render the settings screen (server prefs + client-only form-factor items
      like theme/answer keys), forward save.

---

## 5. Backfill — user stories + tests for existing features   `[ ]`

After the feature work settles, harden the *existing* surface: characterise it
and lock it down with the story→test discipline (see [`docs/process/PLANNING.md`](./PLANNING.md)). This runs
last so feature stages can move fast without a large backfill up front, while
still ending with full regression coverage. New features added in stages 2–4
should still ship *their own* stories + tests inline, not wait for this stage.

- [ ] Inventory every endpoint/feature (the 71 operations + the scheduling
      core) and write a `US-5.<n>` story for each, with acceptance criteria —
      derived from behaviour, not the support matrices.
- [ ] Add a `server/tests/` test for each story's acceptance criteria (using
      the `TestApp` harness from task 1), pinning current behaviour.
- [ ] Add `CruxCore` `update` + `effects()` tests for the client core surface
      so it has full coverage alongside the server.
- [ ] Record any gaps/descope found while characterising (behaviour that is
      undefined or surprising) as follow-up stories, not silently.

**Definition of done:** every endpoint has a `US-5.<n>` story with a passing
`server/tests/` test; no orphan behaviour.

---

## 6. Tags — note/collection tagging   `[ ]`

A teacher/admin-controlled note-level tag system (add, remove, rename, browse,
filter/search) matching Anki's model. The per-student leech work that this stage
originally bundled is **already complete** via US-2.20 (leech is per-student
scheduling state, not a tag) — so stage 6 is now purely the tag system.

### Grounding (Anki reference)

- **Tags are note-level**, not card-level — the `Tags` column is "the note's
  tags", and `tag:` searches notes. (<https://docs.ankiweb.net/browsing.html>,
  <https://docs.ankiweb.net/searching.html>)
- **Tags are hierarchical** with `::` separators (`parent::child`); `tag:animal`
  matches `animal` **and** `animal::mammal` (subtags) — a prefix match, not an
  exact match. `tag:none`, `tag:ani*`, and `tag:re:` also exist.
- **Rename/move rewrites the tag on affected notes** ("renamed the tags … and
  updated your notes accordingly").
- Operations: Add/Remove Tags, a sidebar tag tree, Clear Unused Tags, search by
  `tag:`, and Find-and-Replace within tags.

### Scope & permissions (settled decisions)

- **Tags describe shared note content → teacher/admin-authorable, student
  read-only.** Notes (and their taxonomy) are shared school-wide, so a student
  never authors tags; only `check_teacher_or_admin` may add/remove/rename.
  Students may *see* a note's tags (rendered in the browser), never edit them.
- **No system tags.** Anki's `leech`/`marked` are *not* part of the tag system
  here. `leech` is per-student scheduling state (see `US-6.6`), not a note tag.
- **`marked` is deferred, out of the main experience.** It is a *legacy* Anki
  feature (its manual says "most users will want to use flags instead") and,
  like `leech`, is a per-user signal rather than shared note taxonomy. Do **not**
  model it as a tag now. Revisit it only in the future import/export stage, where
  faithful Anki-data preservation forces an explicit decision on how `marked` is
  represented in Anjuman.
- **Divergence (documented):** Anki lets the single user tag their own notes;
  Anjuman restricts tag *authoring* to teachers/admins because notes are shared
  (same class of divergence as deck-options per-school vs. per-user).

### Design decisions (settled)

- **Storage shape: normalised join table + canonical `tags` table (option a).**
  `tags(id, name)` + `note_tags(note_id, tag_id)`, with the `::` hierarchy carried
  *in the tag name* (so `biology::cells` is one `name` containing `::`). Rename is
  a single row update; prefix/subtag matching (`tag:animal` → `animal` + `animal::*`)
  is `name = x OR name LIKE 'x::%'`. This is chosen over Anki's literal
  space-delimited string per note because it gives cheap rename, a clean distinct-tag
  list, and no per-note string rewrite on renames.
- **Tags are note-scoped (not school-scoped).** A tag attaches to a *note*. If a
  deck's notes are ever shared across schools, their tags travel with the notes
  (organisation is preserved) rather than being pinned to a school. (Contrast with
  decks/presets/note-types, which are school-scoped.) Concrete consequence: the
  `tags`/`note_tags` tables carry **no `school_id`** — tag identity is the `name`,
  and visibility is via the notes the user can already see (the deck-access
  predicate determines which notes, hence which tags, a user sees).
- **`tag:` search scope is deferred to the browser/search stage.** Whether `tag:`
  filters the notes/cards list in place, or is a standalone endpoint, is a browser
  feature decision — not made here. Stage 6 only exposes the *storage + CRUD + a
  distinct-tags list*; search consumes it later.

### Stories (core-first, then shell)

- [x] `US-6.1` — **Schema + contracts.** `tags`/`note_tags` tables + migration;
      `Tag` DTOs in `anjuman_contracts`.
- [x] `US-6.2` — **Tag CRUD on notes.** Add/remove/list tags on a note
      (teacher/admin-gated); a distinct-tags list endpoint (the sidebar tree
      source). `server/tests/` per criterion.
- [x] `US-6.3` — **Rename/move.** Rename a tag (updates the one `tags` row;
      `note_tags` unaffected since it references the tag id). Teacher/admin-gated.
      (Clear-unused is done here: `DELETE /tags` removes orphaned tags.)
- [x] `US-6.4` — **Distinct-tags list + hierarchy.** Expose the distinct tag
      namespace as a `::` tree for the sidebar, and the prefix/`tag:none`/glob
      matching that later search work consumes. `server/tests/`.
- [x] `US-6.5a` — **Client core consumes the tag list + tree.** Standalone
      `Model.tags`/`tag_tree` + `ViewModel` (via `TagView`/`TagNodeView` Facet
      mirrors), events `TagsRequested`/`TagTreeRequested` (+ `TagsResult`/
      `TagTreeResult`), `tags_error`, and clear-on-logout. `shared` update tests.
- [ ] `US-6.5b` — **Per-note tag CRUD in the client (deferred).** Add/remove tags
      on a note from the UI. **Deferred**: the client has no notes/browser model
      yet (the notes screen is ❌ in `docs/support/screens.md`), so there is
      nothing to hang per-note tag actions on. Do this when the notes/browser
      screen lands (future stage).
- [x] `US-6.6` — **Leech as per-student state, not a tag.** Done via US-2.20
      (per-student `student_card_states.leech_at`; `TagOnly` marks, `SuspendCard`
      marks + suspends — no tag-system involvement).

---

## 7. Per-user deck options / personal scheduling   `[ ]`

Allow a student to personalise scheduling for decks they study without mutating
shared, school-scoped presets. Today deck options are school-scoped and
**only teacher/admin-authored** — a deliberate divergence from Anki (where deck
options belong to the single user). This stage makes the personalisation model
**layered overrides** rather than replacing shared presets.

### Why

- Anki deck options are per-user; Anjuman's are per-school. The school preset
  solves *consistency* (a teacher sets `1m 10m` for the whole class), but it
  locks every student of a shared deck to identical scheduling.
- A student adopting a shared deck should be able to set **their own** limits,
  steps, or retention for their study without mutating the teacher's preset.

### Design questions to settle (before implementing)

- [ ] **Override target** — do personal overrides attach to a *deck* or to a
      *preset*? (Per-deck personal options vs. per-user preset clones.)
- [ ] **Storage** — a `user_deck_options(user_id, deck_id, …)` table, per-user
      preset clones, or a delta-compare model?
- [ ] **Permission model** — a `decks.allow_personal_options BOOLEAN` so a
      teacher can choose per-deck whether students may override. Turns today's
      hard "school-only" rule into a per-deck choice. *(Coordinates with
      stage 9: the "may this user override" capability should come from the
      consolidated authorization model, not a new ad-hoc role check.)*
- [ ] **Fork vs. inherit** — does a personal override fork a snapshot of the
      preset (deviating independently), or diff against the live preset?
- [ ] **Analytics/comparison** — how do diverging student options affect class
      analytics and cross-student comparison?
- [ ] **Interaction with daily limits** — personal "new-per-day" limits overlap
      with the per-user daily-limit prefs; resolve before finalising both to
      avoid two competing mechanisms.

### Definition of done

- Effective options resolve as *personal override if present, else school
  preset*, with an explicit per-deck permission to admit overrides.
- A student can study a shared deck with their own limits/steps without
  touching the teacher's preset.
- Documented in [`docs/support/deck-options.md`](../support/deck-options.md) "Key architectural differences".

---

## 8. History / Undo   `[ ]`

Let users see what changed and (optionally) reverse their actions. **This stage
is scoped but *not* decided** — the design questions below are deliberately left
open until the stage is tackled, so no commitment is made here beyond the
boundary of the feature and the shape of the work. Add this to the stage and
plan it properly (story list + acceptance criteria) when it becomes current work.

### Scope (the boundary, fixed)

- **What it covers:** recording user actions and, if pursued, reversing them. The
  obvious candidates are deck management (rename/delete/assign/options), note/card
  editing, card actions (flag/suspend/bury/reschedule), preset edits, class/roster
  changes, and study reviews.
- **What it is *not*:** a generic event-sourcing rewrite of the domain. The
  existing row-per-entity model (`decks`, `notes`, `cards`, `student_card_states`,
  `deck_options`, …) stays; history is a *side table*, not a replacement.
- **Existing assets to build on:** `reviews` already records `rating` /
  `state_before` / `reviewed_at`, and `student_card_states` holds the per-student
  scheduling row (`stability`, `difficulty`, `due_at`, `reps`, `lapses`,
  `step_index`, `suspended`, `buried_at`) — the raw material for reversing study.
  There is currently **no** audit/history table anywhere.

### Design questions to settle (deferred — do not decide now)

- [ ] **Undo vs. browse** — is the goal Anki-style shallow *undo*, a browsable
      *activity log*, or full multi-step *rollback*? These are different features
      with very different cost; do not conflate them.
- [ ] **Which actions are reversible?** In particular: **does undo include
      study/reviews?** Reversing an FSRS review is the hard, risky case (must
      restore the exact prior `student_card_states` row and delete the matching
      `reviews` row, and a mid-stream delete can perturb downstream scheduling).
      Descoping review-undo initially is the cheap path. Note: bury/suspend are
      `card_mod` mutations with *existing inverse endpoints* (`unbury`/
      `unsuspend`, card and note variants), so they are cheaply reversible — but
      they currently have **no in-UI undo hook** (see US-4.5b, deferred for this
      stage).
- [ ] **Persistence vs. ephemeral** — persistent (history table, survives
      reload) vs. Anki-style in-session undo (ring buffer, gone on reload).
- [ ] **Scope/permission** — personal history only, or school/class history
      visible via the existing grant model? Should reuse the US-4.7/2.19 deck
      access predicate rather than invent a second one.
- [ ] **Granularity** — whole-entity `before`/`after` snapshots (cheap) vs.
      per-field diffs (nicer UI, more code).

### Likely decomposition (tentative — refine when planning the stage)

- [ ] `US-8.1` — `history` append-only table (id, school/user, entity type + id,
      action, `before`/`after` JSONB, `created_at TIMESTAMPTZ`), written in the
      *same transaction* as each mutation.
- [ ] `US-8.2` — `GET /history` read endpoint (filter by user/entity/time) +
      the client history screen (core first, then shell).
- [ ] `US-8.3` — undo for *non-scheduling* mutations (restore the `before`
      snapshot).
- [ ] `US-8.4` — undo for a single review (only if the "include reviews" decision
      is yes; needs a prior-states snapshot + a documented FSRS caveat).
- [ ] `US-8.5` — diff/rendering polish (what changed, per-field).

### Reference / divergence notes (record when implementing)

- Anki exposes *undo* as single-level and in-memory; a persistent, multi-tenant,
  stepped history is a deliberate divergence (same class as the deck-options
  per-school vs. per-user split) and must be documented against an Anki reference.
- FSRS scheduling is stateful — document any limitation that multi-step review
  undo is not reconstructable for free, the same way the stateless-gather/sort
  and deterministic-seed divergences are recorded in
  [`docs/support/deck-options.md`](../support/deck-options.md).

---

## 9. Permissions — consolidated authorization (RBAC → PBAC seam → ReBAC)   `[x]`

Consolidate the organically-grown authorization into a single, scalable model
behind a capability seam. Today authorization is three overlapping layers: a
centralized JWT `role`, ~40 hand-rolled `if role` checks spread across ~9
handlers, and the US-4.7 deck-grant module. This stage makes them **one** model
with a documented decision. See [`docs/adr/permissions-model.md`](../adr/permissions-model.md) for the
full analysis and the models comparison (RBAC / PBAC / ABAC / ReBAC).

### Settled decisions (locked)

- **Naming:** resource-first capability names (`can_manage_decks`,
  `can_manage_notes`, …).
- **Capability list:** the exact table below (refine later, not now).
- **Deck governance:** two **orthogonal dimensions** (corrected during US-9.1 —
  governance and study are *not* one ordering):
  - `DeckPerm` (study) = `None | VisibleOnly | Study` (unchanged from US-4.7).
  - `DeckGovernance` (management) = `None | Contribute | Manage`;
    `Manage` = owner (teacher/admin); `Contribute` = collaborator
    (teacher/admin). Governance is **role-gated**, study is not.
- **No admin trump for decks (revised 2026-10-07):** admins are held to the same
  owner/collaborator relationship rules as teachers — `Manage` = owner,
  `Contribute` = collaborator. The `Admin` role alone grants neither study, nor
  governance, nor deck-list/note-list visibility on any deck the admin does not
  own or collaborate on. (School/user admin + class management stay
  admin-permissive.)
- **Incremental:** one story per capability group, each independently green.
- **No schema change; no ABAC; `role` stays in the JWT** (coarse fast-path).

### The model (two steps, never conflated)

Every authorization decision is: **capability (role bundle) AND relationship
(user ↔ specific resource)**.

```
allow  if  capability(role)                 // "may this kind of user do this kind of action at all?"
         AND relationship(user, resource)   // "…to *this specific* resource?"
```

A role does not grant access to a *particular* resource; it only says *what kind
of action* that role may take *on resources it is related to* in a specific way.
There is **no admin trump for decks** — admins follow the same relationship rules
as teachers.

### Capability list (exact)

| Capability | Role bundle | Relationship gate | Backed by today |
|---|---|---|---|
| `can_administer_school` | `{Admin}` | none (school-wide) | `check_admin` |
| `can_manage_users` | `{Admin}` | none (school-wide) | `admin_users.rs` |
| `can_manage_classes` | `{Admin, Teacher}` | none (school-wide) | `classes.rs` |
| `can_manage_note_types` | `{Admin, Teacher}` | none | `note_types_handler.rs` |
| `can_manage_presets` | `{Admin, Teacher}` | none | `deck_options_handler.rs` |
| `can_manage_tags` | `{Admin, Teacher}` | none | `tags.rs` |
| `can_manage_deck` | `{Admin, Teacher}` | owner **or** admin (this deck) → `Manage` | `check_deck_owner` |
| `can_manage_deck_content` | `{Admin, Teacher}` | owner **or** collaborator **or** admin (this deck) → `Contribute` | `check_deck_collaborator` |
| `can_view_deck_administration` | `{Admin, Teacher}` | owner/collab/admin (this deck) | `is_deck_collaborator` |
| `can_study_deck` | any authenticated | grant over subtree (recursive) → `Study` | `deck_permission` |
| `can_act_on_own_card` | any authenticated | own `student_card_states` row | card_mod (no gate today) |
| `can_view_dashboard` / `can_view_analytics` | teacher/admin vs student shape | none | inline `role == Student` |

### Stories (incremental, each green)

#### US-9.1 — Primitives + capability module (no behavior change)

- [x] Add `DeckGovernance::{ None, Contribute, Manage }` (orthogonal to the
      study `DeckPerm::{ None, VisibleOnly, Study }`); `deck_governance()` returns
      `Manage` (owner/admin or admin-trump) / `Contribute` (collaborator) gated by
      the role bundle.
- [x] Add role-bundle capability fns (pure `Claims → bool`):
      `can_administer_school`, `can_manage_users`, `can_manage_decks`,
      `can_manage_notes`, `can_manage_presets`, `can_manage_note_types`,
      `can_manage_tags`, `can_manage_classes`, `can_view_dashboard`,
      `can_view_analytics`; plus `require_teacher_or_admin` / `require_admin`
      gates preserving the per-resource deny messages.
- [x] Tests (named): 8 in `server/tests/permissions_capabilities.rs` (role-bundle
      predicates + governance levels + student-not-governance). Existing US-4.7
      `deck_permissions.rs` unchanged — proving study semantics preserved.

#### US-9.2 — Deck governance through the relationship model

- [x] Make `check_deck_owner` = `require deck_governance == Manage`; make
      `check_deck_collaborator` = `require deck_governance ∈ {Contribute, Manage}`
      (thin adapters; deleted their duplicated owner/collab SQL; preserved the
      legacy 404/403 status + messages).
- [x] Preserve `move_card`'s **target-deck-only** collaborator check as-is
      (pre-existing; did not add a source-deck check).
- [x] **Tests:** 8 parity tests in `server/tests/deck_governance_adapters.rs`
      (owner/collaborator/admin/stranger/missing-deck for both helpers).

#### US-9.3 — Sweep the remaining inline role checks

- [x] Convert the 6 duplicated `check_teacher_or_admin` definitions (decks,
      deck_options_handler, classes, notes, note_types_handler, tags) and
      `check_admin` (admin_users) to thin delegates over
      `permissions::require_teacher_or_admin` / `require_admin`, preserving each
      resource's deny message.
- [x] Convert the inline `claims.role == Student` deny-gates in `search_users`,
      `analytics` (`check_class_access`), `dashboard` to `require_teacher_or_admin`
      / `can_view_analytics` / `can_view_dashboard`. (`study` already used
      `check_deck_studyable` — no change.)
- [x] Dropped the now-unused `UserRole` contract imports from the six converted
      handler files (build is warning-free).
- [x] Existing `server/tests/` cover the student-denied parity (tags, deck
      permissions, teacher-admin study); full suite green.

#### US-9.4 — Backfill + DoD gate

- [x] Converted the remaining `claims.role == Admin` *query-shaping* branches in
      `dashboard`/`classes`/`decks`/`notes`/`is_deck_collaborator` to
      `can_administer_school`/`can_view_*`/`deck_permission`. The two leftover
      `claims.role == Teacher|Student` checks in `list_decks`/`deck_counts` are
      **tri-partite role dispatch** (three disjoint queries), intentionally kept
      as explicit role checks — not boolean capabilities.
- [x] `grep`-asserted exactly one definition of `require_teacher_or_admin` and
      `require_admin` (both in `permissions.rs`); zero remaining `match claims.role`
      bodies anywhere.
- [x] `server/tests/` named test per capability (role-bundle predicates incl.
      `can_view_dashboard`/`can_view_analytics`, governance levels).
- [x] OpenAPI route coverage unchanged (no routes added/removed); `cargo build`/
      `cargo test` green; no new warnings.

#### US-9.5 — Expose server-resolved permissions to the client (feature gating)

- [x] Contracts: add `auth::Permissions` (the 10 role-bundle capability
      booleans) + `UserResponse.permissions`.
- [x] Server: `permissions::permissions_for(role) -> Permissions` (authoritative
      role → capability mapping); populated by `login`/`me`/`create_user`.
- [x] Core: `ViewModel.permissions: PermissionsView` (Facet mirror) mapped from
      `UserResponse.permissions`; defaults when unauthenticated. Study is *not*
      here — it stays per-deck `DeckSummary.studyable`.
- [x] Tests: `shared` `permissions_map_into_view` (student vs. teacher caps).
- [ ] **Shell contract:** gate management affordances (create deck, presets,
      tags, classes, note types, user admin, dashboard/analytics) on
      `vm.permissions.*`; keep study gated on `deck.studyable`. Also remove/replace
      the hard-coded dev login buttons if desired (they're dev-only). **(shell
      agent scope — a follow-up.)**

### Remaining open (not blocking — resolve only if it bites)

- [ ] **Membership second axis** — `class_members.membership_role` (teacher/student
      *within* a class) still overlaps the global `users.role`. Kept distinct for
      now; revisit when class-scoped management appears.
- [ ] **Role in JWT vs. DB** — settled as "keep `role` in the JWT" for this pass;
      the capability seam makes a later DB-backed move a one-function swap.

### Notes / ordering

- **Prerequisite** for stage 7 (personal deck options needs a "who may override"
  permission → `can_manage_deck`-or-`can_study_deck` + a new per-deck flag) and
  threads through stage 6 (tag/browser). Placed as stage 9 to keep the backlog
  ordering stable; may be pulled forward once stage 7 stops being deferred.
- **No ABAC** — explicit non-goal; revisit only for per-school-configurable policy.

---

## Notes

- **Ordering rationale:** task 1 is invisible-but-risky infrastructure (no API
  change) done *before* the feature work so that tasks 2 & 3 don't have to be
  migrated twice. Tasks 2 & 3 change the API, which task 4 then consumes.
  Task 5 (backfill) runs last so the existing surface gets full story+tests
  coverage *after* the feature work settles, rather than front-loading it.
  Task 6 (tags) is a deferred cross-cutting feature that revisits the leech
  `TagOnly` no-op from US-2.3. Task 7 (per-user deck options) makes scheduling
  personalisation layered (personal override → school preset) instead of
  Anki's strict single-user model. Task 8 (history/undo) is cross-cutting.
  Task 9 (permissions) consolidates authorization behind a capability seam and
  is a prerequisite for task 7; it is staged last only to keep the backlog
  ordering stable and may be pulled forward when task 7 becomes current work.
- **Not implemented: full Anki gather→sort parity** — true parity still needs
  a two-phase gather-then-sort pipeline, which contradicts our stateless,
  sessionless study design, so we approximate with gather-primary sort-secondary
  ordering in a single query. See [`docs/support/deck-options.md`](../support/deck-options.md) "New-card ordering
  composes as gather then sort". (This limitation is narrower than it once
  seemed: see the "Mix" finding below — the mix itself is not a queue concern.)
- **"Mix with reviews" is Anki's stateless `Intersperser`, now implemented.**
  Anki's `mix` (both `new_review_order` and `interday_order`) is implemented in
  `rslib/src/scheduler/queue/builder/intersperser.rs` (`Intersperser`), which is a
  pure ratio-based even-distribution of two already-sorted iterators — **not** a
  due-date merge, and **no** materialized queue is needed. The decision to draw
  from queue A vs B next is a function of four integers (`a_len`, `b_len`,
  `a_idx`, `b_idx`):

  ```rust
  ratio = (a_len + 1) as f32 / (b_len + 1) as f32;
  // take b next iff (b_idx + 1) * ratio < (a_idx + 1)
  ```

  Implemented as `intersperse_draw_b` in `study.rs`, driven by `seen_today`
  (new/review/interday seen) + the due-now class counts. `mix` is restored as
  the default, matching Anki.
- **Three-bucket counts, persisted via `reviews.interday`.** Anki gathers
  interday-learning and review cards against the **same** `LimitKind::Review`
  counter (interday gathered first) — matching our shared review budget — but
  still tracks them as **separate counts** (`learning = intraday + interday`,
  `review`, `new`) and interleaves them with **two nested `Intersperser`s**:
  first interday-vs-review, then new-vs-(review+interday). To be fully faithful
  (and immune to later step edits), we persist an `interday` flag on `reviews`
  at review time (migration 0012) and split `seen_today` into
  new/review/interday counters. The due-now class counts used for the ratio are
  *physical* (pre limit-clamp), so `mix` is exact when limits are not binding
  and a close approximation when they are.
- **FSRS simulators are descoped to a post-client stage.** Anki has two distinct
  simulators behind the "Help Me Decide (Experimental)" and "FSRS Simulator
  (Experimental)" buttons (they show different graphs). Both are UI-heavy and
  may need server-side simulation endpoints, so we do no server work for them
  now; revisit after stage 4 (client). The optimizer and "check health" are
  separate concerns and are not blocked by this descope.
- Update this file's checkboxes (`[ ]`→`[x]`) and status markers at the start
  and end of every sub-task, with a one-line note of what changed.
