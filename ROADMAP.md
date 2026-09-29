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
- [x] **FSRS parameter optimization** → US-2.18a (store+consume, done) + US-2.18b (optimizer, deferred).
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

      **Out of scope / decision needed**
      - "Check health when optimizing" (collection-wide, stage 7) and the
        simulators (post-client).
      - Storage scope is per-preset columns on `deck_options` (Anki: per-preset
        shared across the collection — recorded).
      - Search-filter and `ignore_revlogs_before_date` semantics need Anki
        fidelity research before implementing.
- [ ] Cross-check the remaining ❌/🟡 rows and either implement or consciously
      descope each (record the descope decision in the matrix).
- [ ] Cross-check the remaining ❌/🟡 rows and either implement or consciously
      descope each (record the descope decision in the matrix).

Each item must keep the OpenAPI spec in sync (new/changed DTOs → regenerated
`/api-docs`).

---

## 3. Preferences — feature-complete with Anki   `[ ]`

Close the gaps in `server/PREFERENCES_SUPPORT.md`. The backend currently
persists only two prefs (`day_start_hour`, `learn_ahead_seconds`) and has
**no endpoint** to read/update them. The matrix's "Summary of the biggest
gaps" is the working list:

- [ ] **Add preferences CRUD** — `GET/PATCH /preferences` (per-user), using the
      existing `user_preferences` table (or a generalized key/value row).
- [ ] **Timebox time limit** — implement the missing timebox preference.
- [ ] **Audio-related preferences** — depends on adding audio support (media
      storage/serving); likely a larger cross-cutting item — decide scope.
- [ ] **Editing conveniences** — "default deck" persistence (last-used
      deck/note type); accent-insensitive search (or fold into a search-token
      improvement).
- [ ] Timezone correctness — the current day-boundary is UTC-anchored via
      `day_start_hour`; consider a per-user timezone column.
- [ ] Cross-check the remaining ⚪/❌ rows and record descope decisions in the
      matrix.

---

## 4. Frontend — implement the client   `[ ]`

Replace the counter-demo core with the real domain, consuming
`anjuman_contracts` and the server API.

- [ ] Add `anjuman_contracts` as a path dependency of the client `shared` crate
      (and decide whether `utoipa`'s `openapi` feature is needed client-side —
      it should **not** be).
- [ ] Model the client domain in `client/shared/src/app.rs`:
      - [ ] `Model` (auth state, decks, notes, cards, study session, classes).
      - [ ] `Event`, `ViewModel`, `Effect` (add `crux_http` + `crux_kv` +
            `crux_time` capabilities — the core is currently `Render`-only).
      - [ ] Consume `anjuman_contracts` DTOs for HTTP payloads.
- [ ] Implement the auth flow (login → store JWT → `/me`).
- [ ] Implement stable screens first: decks, notes, cards, study — then fold in
      deck-options/preferences UI once tasks 2 & 3 land.
- [ ] Keep the FFI `Bridge`/`codegen` surface working as the model grows;
      regenerate bindings.
- [ ] `cargo build`/`cargo test` + `trunk serve` succeed.

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
