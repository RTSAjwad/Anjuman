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

> Detailed plans: `docs/postgres-migration.md`, `docs/timestamptz-migration.md`.

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
> and per-row classification live in `DECK_OPTIONS_SUPPORT.md` ("Scope: server
> vs. client behaviour").

Close the gaps in `server/DECK_OPTIONS_SUPPORT.md` as far as practical. The
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
- [x] **Lapses** — leech threshold/action + empty relearning steps. Done
      (US-2.2–US-2.4). Minimum interval is SM-2 — out of scope.
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
      - Tagging the note (both actions tag in Anki) — blocked on a tag system
        (ROADMAP stage 6). `TagOnly` is a no-op until tags land.
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
      learning/relearning Hard rules. See `PLANNING.md` §2.

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

      Options (from `DECK_OPTIONS_SUPPORT.md` "Audio" / "Timers" / "Auto Advance"):
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
      (stage 7), leech tagging (stage 6), Easy Days scheduling (interval load
      balancer), US-2.5 (empty learning steps, experimental), US-2.18b
      (optimizer). Fixed two stale rows: "Display order taken from selected
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

## 3. Preferences — server-side only   `[x]`

> **Scope:** most Anki preferences are client-side, form-factor-specific, or
> Anki-specific and do **not** belong in the server (see `server/PREFERENCES_SUPPORT.md`
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
  `PREFERENCES_SUPPORT.md`). A future stage may revisit a school-wide default.
- **Storage format**: IANA name (`TEXT`), not a fixed `UTC+hh` offset — a fixed
  offset silently breaks across daylight-saving changes, whereas the IANA name
  follows the zone's own offset transitions.
- **Default**: `"UTC"` for now, preserving current behaviour exactly until a
  client/onboarding sets a real zone. Revisit later (school-based, location-
  based, etc.). **Decision recorded in `PREFERENCES_SUPPORT.md`.**
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
2–3: a story derives from `client/SCREENS_SUPPORT.md` (the feature matrix), and
every acceptance criterion maps to a `CruxCore::update` test in `client/shared`
(see `PLANNING.md` §3 for the client test idiom — assert on `caps.effects()` and
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
      predicate is US-4.7 and teacher/admin study + real counts is US-4.8
      (drafted, next); notes + cards remain.
- [ ] **Deferred client-side behaviours** — now in scope: on-screen timer, audio
      playback, auto-advance (US-2.14), timebox popup (US-3.2), leech "Tag Only"
      popup, `response_time_ms` stopwatch (US-2.15), theme/answer-key bindings
      (see `client/SCREENS_SUPPORT.md`). Each gets its own story.
- [ ] **Deck-options/preferences UI** — the settings screens that CRUD
      `deck-options` and `/preferences`.
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
counter tests. This story front-loads the API surface (per `PLANNING.md` §4) so
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
`client/AGENTS.md` §7 and `SCREENS_SUPPORT.md` "Shell coverage").

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
- [x] Each criterion has a `shared` test named after it; `cargo test` passes.

**Shell contract**

- [ ] When a deck is selected, forward `Event::StartStudy`.
- [ ] Render the current card (front; reveal back on demand), plus the four
      answer buttons (Again/Hard/Good/Easy) forwarding `Event::Answer { rating }`.
- [ ] Render the counts and the "nothing due" completion state, using
      `ViewModel.busy` to show "loading" while `busy && current_card == None`.

**Out of scope**

- Flip/reveal animation, bury/suspend-from-study, flags, and any
  offline/queueing. Predicted-interval labels are now in scope: `StudyCardView`
  exposes `predicted_interval` (rating→seconds) for the shell to format under
  each answer button (formatting itself is shell polish).

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
      (dark/light/follow-system)` in `SCREENS_SUPPORT.md` (a ⚪ shell-only
      behaviour) — the shell applies the inversion class/stylesheet in the
      iframe, not the core/server. This story documents the *decision*; the shell
      work is a separate ⚪ cell, not a `shared` criterion.

**Out of scope**

- The shell's actual inversion implementation (the `Theme ⚪` cell), template
  editor UI for editing `styling`, and per-template CSS.

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

- [ ] The deck-access predicate exists and answers `Study` for a deck the user
      owns, collaborates on, or reaches via a class (extended over the subtree
      per US-2.19) — and `Study` is **not** granted merely for being `Admin`.
- [ ] `check_deck_studyable` (and study `GET`/`POST`) consult the predicate; an
      admin who does not own/collaborate a deck gets `403` (behaviour change from
      today's blanket grant).
- [ ] `has_grant` no longer short-circuits `Admin → true`.
- [ ] The predicate is `pub` and reusable from `list_decks`/`deck_counts` so
      US-4.8 can compute `studyable` from it (no duplicate logic).
- [ ] Each criterion has a `server/tests/` test named after it (e.g.
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

- [ ] `list_decks` admin + teacher branches compute `studyable` from the new
      deck-access permission predicate (US-4.7) — i.e. `Study` is granted —
      instead of hardcoding `true`, and are scoped to the decks the caller
      actually holds a grant on (not every school deck).
- [ ] `list_decks` admin + teacher branches return real per-state counts by
      calling `deck_counts_for_student(claims.sub, deck_id)` (and real
      `total_count`), so new/learning/review/relearning are populated —
      mirroring the student branch.
- [ ] `GET /decks/counts` teacher/admin branch returns per-state counts (via the
      same `deck_counts_for_student`) instead of hardcoded `0`.
- [ ] `studyable` and counts are consistent: a deck marked studyable in the list
      is one the `Study` permission (US-4.7) admits, and its counts match what
      the study flow would show.
- [ ] Each criterion has a `server/tests/` test named after it (e.g. in a new
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

## 5. Backfill — user stories + tests for existing features   `[ ]`

After the feature work settles, harden the *existing* surface: characterise it
and lock it down with the story→test discipline (see `PLANNING.md`). This runs
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

A general tag system (note-level tags: add, remove, filter, browse/search by
tag). This is a cross-cutting feature in its own right, deferred until after the
core feature stages settle.

- [ ] `note_tags` table (or equivalent) + schema migration.
- [ ] CRUD + list/browse/search-by-tag endpoints on `anjuman_contracts`.
- [ ] Client (stage 4) consumes tags in the browser/search UI.
- [ ] **Revisit leeches** — wire the leech `TagOnly` action (currently a
      documented no-op, see US-2.3) to actually emit the `leech` tag, and back
      the `SuspendCard` action's "also tag the note" behaviour. Today leech
      le marks the note via `notes.leech_tagged_at` only.

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
      hard "school-only" rule into a per-deck choice.
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
- Documented in `DECK_OPTIONS_SUPPORT.md` "Key architectural differences".

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
  Anki's strict single-user model.
- **Not implemented: full Anki gather→sort parity** — true parity still needs
  a two-phase gather-then-sort pipeline, which contradicts our stateless,
  sessionless study design, so we approximate with gather-primary sort-secondary
  ordering in a single query. See `DECK_OPTIONS_SUPPORT.md` "New-card ordering
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
