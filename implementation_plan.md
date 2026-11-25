# Anki-Compatible Flashcards Studio & 1-to-1 Scheduling Sync

Replace the quiz-generated SRS page and quiz-to-Anki export button with a full Anki-compatible Flashcards workspace powered by shared GitHub-hosted `.apkg` decks, retaining and upgrading the FSRS-6 engine for 1-to-1 Anki scheduling interoperability, full curriculum `.apkg` export, scoped `.apkg`/`.colpkg` scheduling import, and Firestore-backed cross-device progress sync.

---

## User Review & Critical Decisions

> [!IMPORTANT]
> All core product decisions below reflect your Phase 1 selections. Please review the remaining configuration details (such as the default GitHub repository pointer for shared `.apkg` files) before approving execution.

- **Confirmed — Shared APKG Hosting**: Shared `.apkg` curriculum decks will be fetched from a centralized hosted GitHub repository (mirroring the existing Quizzes and Supplements GitHub integration pattern so every scholar accesses the exact same `.apkg` deck files). *Default*: A dedicated central config block (`AnkiDecksConfig`, e.g., `Yuzuru-Yamazaki/anki-decks` on `main`, supporting both an index manifest and recursive `.apkg` tree discovery) that you can point to any repository/branch in one spot.
- **Confirmed — FSRS-6 Kept & Elevated**: Rather than removing `FSRS-6`, we keep and extend it so every card maintains both full FSRS-6 memory state (`stability`, `difficulty`, `desiredRetention`, `reps`, `lapses`, `lastReview`, `due`) and exact Anki SQLite scheduling equivalents (`type`, `queue`, `due`, `ivl`, `factor`, `left`, `flags`, `data`).
- **Confirmed — Scoped Scheduling Import**: Users can upload an Anki `.apkg` or `.colpkg` export (exported with scheduling included). Scheduling is imported **strictly for cards whose identity matches cards hosted on the website**, while any unrelated personal cards in the user's Anki package are safely ignored and reported in the import summary.
- **Confirmed — Firestore Persistence & Quiz-to-Anki Removal**: Per-user Anki scheduling, deck study options, and daily review counters are persisted in Firestore (with IndexedDB local caching for fast offline-first reviews and media blobs). The old "Export Anki Deck (.apkg)" button on quiz result screens and automatic creation of flashcards from quiz mistakes are removed.

---

## 1. Overview & Core Concept

- **What It Does**: Transforms the Flashcards tab (formerly `SRS`) into a full-featured Anki client built directly on shared `.apkg` curriculum packages. Scholars study hosted decks and nested `::` subdecks using FSRS-6 spacing, inspect cards in a searchable browser, customize deck study limits and flags, export the entire curriculum with intact 1-to-1 Anki scheduling, and re-import scheduling updates back from desktop/mobile Anki exports.
- **Target Audience**: Students studying a unified, shared curriculum who want genuine Anki compatibility (cloze deletions, media, templates, flags, deck limits, FSRS-6 memory state) both on the web and seamlessly synced with Anki desktop/AnkiMobile/AnkiDroid.
- **Key Value**: Eliminates artificial conversion from multiple-choice quizzes while unlocking rich Anki flashcards, exact round-trip `.apkg` scheduling portability, and persistent per-user Firestore progress without losing existing FSRS-6 retention math.

---

## 2. User Experience & Visual Design

### Key User Flows

1. **Deck Tree & Study Overview (Home of Flashcards Tab)**
   - Opens directly to the shared curriculum deck hierarchy (nested `folder::subdeck` tree with expand/collapse chevrons and `New / Learn / Due` numeric counters using `tabular-nums`).
   - Top workspace toolbar offers four real controls: **Study Due**, **Import Anki Scheduling (`.apkg` / `.colpkg`)**, **Export Entire Curriculum (`.apkg`)**, and **Deck Refresh**, plus quick switches between **Decks**, **Card Browser**, **Deck Options**, and **Statistics**.
   - If the remote `.apkg` repository is empty or unreachable, displays an informative state explaining where shared `.apkg` decks are loaded from alongside a manual refresh action and immediate scheduling import availability once hosted decks exist.

2. **Active Card Study Session (1-to-1 Anki Study Stage)**
   - **Front / Question Stage**: Renders card HTML templates, Cloze `[...]` masks, type-in-the-answer input (`{{type:Field}}`), embedded `.apkg` images, inline `[sound:...]` audio controls, and KaTeX math expressions inside a clean, distraction-free central study surface.
   - **Action Strip**: Top utility row shows Deck title, Card State (`New` / `Learning` / `Review` / `Relearning`), Flag picker (7 Anki flags), **Suspend**, **Bury**, **Undo Last Review**, and **Card Info**.
   - **Answer & FSRS-6 Rating Bar**: Pressing `Space` or clicking **Show Answer** reveals the back side / cloze answers (with character-by-character diff for type-answer cards) and presents the four Anki/FSRS-6 rating buttons (**Again `1`**, **Hard `2`**, **Good `3`**, **Easy `4`**) with live projected intervals (`<1m`, `10m`, `4d`, `2.3mo`).

3. **Card Browser, Custom Study, Options & Statistics**
   - **Card Browser**: Filterable table supporting Anki search queries (`deck:`, `tag:`, `flag:`, `is:due`, `is:new`, `is:learn`, `is:review`, `is:suspended`, `is:buried`, `is:leech`) with right-side card preview, review log history (`revlog`), FSRS memory state inspector (`S`, `D`, `Retrievability %`), and bulk actions (flag, suspend/unsuspend, bury, reset/forget, custom study filtered deck).
   - **Deck Options**: Configurable per deck or preset: daily New cards limit, max Reviews/day, FSRS Desired Retention (`0.70–0.97`), learning & relearning step minutes, new card gather/sort order, max interval cap, leech threshold, and answer timer cap (`maxTaken`).
   - **Statistics**: Clean academic breakdown of card maturity (New / Learning / Young / Mature / Suspended), 30-day review forecast, true retention percentage, and stability histogram.

4. **Curriculum Export & Scoped Anki Scheduling Import**
   - **Export Entire Curriculum (`.apkg`)**: Bundles all hosted curriculum decks, notes, cards, HTML/CSS templates, media files, `revlog` entries, and every card's live 1-to-1 Anki scheduling state into a single valid `.apkg` archive ready to open directly in Anki.
   - **Import Scheduling (`.apkg` / `.colpkg`)**: User drops or selects their exported `.apkg` or `.colpkg` file. A preview modal parses the database, matches cards strictly against the hosted site curriculum (`guid + ord`, with safe field-checksum confirmation when needed), and shows exact match counts (`Matched Hosted Cards`, `Updated Schedules`, `Skipped Non-Site Cards`) before applying changes to Firestore and local state.

### Visual Identity & Layout Architecture

- **Theme Alignment**: Inherits the active Desperado theme CSS custom properties (`--md-surface`, `--md-surface-container`, `--md-primary`, `--md-on-surface`, `--md-outline-variant`, `--md-success`, `--md-error`) so custom light/dark/editorial themes remain harmonious across every Anki card and control.
- **Typography & Numbers**: Preserves the app's configured display and body typefaces (`Plus Jakarta Sans`, `DM Sans` / theme font) paired with `JetBrains Mono` (`tabular-nums`) for all deck counts, intervals, FSRS stability/difficulty metrics, and search filters.
- **Clean Hierarchy & Restraint**: Replaces rounded pill metadata clutter with crisp unboxed typographic separators (`·` / `/`), keeps 60% neutral study canvas / 30% structural panels / 10% primary/semantic accents (Blue = New, Coral/Red = Learn/Again, Emerald = Review/Easy, Amber = Hard), avoids thick left borders and decorative status dots, and ensures single-line controls (`whitespace-nowrap`) with visible focus rings and keyboard-first ergonomics.

---

## 3. Key Product Decisions & Trade-Offs

- **Decision 1: Dual Anki Database Parser & Writer (`anki21b` + `anki21` / `anki2`)**
  - *Chosen Approach*: Support reading both modern Zstandard-compressed Protobuf `.apkg`/`.colpkg` (`collection.anki21b` + binary `media`) and classic SQLite JSON archives (`collection.anki21` / `collection.anki2` + JSON `media`) fully in-browser via `jszip`, `sql.js`, and a lightweight browser Zstandard decompressor + Protobuf wire decoder. For curriculum `.apkg` exports, emit standard `collection.anki21` / `collection.anki2` + `collection.anki21b`-compatible SQLite fields including JSON `cards.data` (`{"s": stability, "d": difficulty, "dr": desiredRetention}`) so Anki Desktop (with or without FSRS enabled) imports 100% of cards, media, revlogs, and exact scheduling with zero loss.
  - *Why*: Anki 2.1.50+ defaults to `collection.anki21b` unless "Support older Anki versions" is checked; supporting both guarantees users never hit corrupt-file errors when importing scheduling from modern Anki.
  - *Alternatives Considered*: Supporting only legacy `collection.anki2` was rejected because most desktop Anki users export `.anki21b` by default.

- **Decision 2: Strict Hosted-Card Identity Matching (`notes.guid` + `cards.ord`)**
  - *Chosen Approach*: Every hosted `.apkg` card has a canonical identity key derived from its hosted Anki `notes.guid` and `cards.ord` (plus normalized note-content fingerprint validation). During `.apkg`/`.colpkg` scheduling import, only records matching a card currently present in the hosted repository are accepted and persisted; any personal decks or custom cards from the user's Anki profile are filtered out and counted as skipped.
  - *Why*: Enforces the strict requirement that foreign cards never leak into the site curriculum while making scheduling round-trips deterministic even when Anki reassigns internal local `nid`/`cid` timestamps on import.
  - *Alternatives Considered*: Matching purely by numeric `card.id` (`cid`) was rejected because Anki can regenerate integer IDs if a user re-imports decks across profiles.

- **Decision 3: Chunked Firestore Persistence + IndexedDB Media Cache**
  - *Chosen Approach*: Store hosted `.apkg` deck binaries/parsed cards and binary media assets (`<img>` / `[sound:...]`) locally in IndexedDB (fetched and refreshed from the shared GitHub repository), while syncing each user's per-card scheduling states, review logs summary, and deck options in Firestore under `users/{uid}/ankiDecks/{deckKey}` (partitioned/chunked compact maps per hosted deck file) plus lightweight summary counters (`dueCardsCount`) for navbar/dashboard indicators.
  - *Why*: Individual Anki cards can number in the thousands, and Firestore has a 1 MB per-document ceiling plus read/write quota considerations. Partitioning user scheduling state by deck chunk in Firestore subcollections avoids document bloat, minimizes read/write operations during study sessions, and preserves security rules isolation per user (`isOwner(uid)`).
  - *Alternatives Considered*: Saving one Firestore document per card (too many read/write ops on large decks) or stuffing all cards into the root `users/{uid}` profile document (risks hitting the 1 MB document limit).

- **Decision 4: Unified FSRS-6 & Anki Queue State Machine**
  - *Chosen Approach*: Keep FSRS-6 (`stability`, `difficulty`, `requestRetention`) as the core memory-state calculator inside a full Anki card-state machine (`New -> Learning -> Review -> Relearning`, supporting intra-day learning steps, Anki epoch `crt`-based `due` day calculation for review cards, ms-accurate timestamps for learning cards, sibling burying, manual suspend, leech detection, flags, and undo history).
  - *Why*: Preserves the superior FSRS-6 spacing accuracy requested while guaranteeing that `type`, `queue`, `due`, `ivl`, `factor`, `reps`, `lapses`, `left`, and `cards.data` match Anki Desktop 1-to-1 on both export and re-import.

---

## 4. Technical Architecture & Data Strategy *(Technical Reference)*

### Architecture & Component Hierarchy

```text
┌───────────────────────────────────────────────────────────────────────────┐
│                 GitHub Hosted .apkg Curriculum Repository                 │
│     (Shared manifest / Git Trees discovery -> versioned .apkg blobs)      │
└───────────────────────────────┬───────────────────────────────────────────┘
                                │ fetch + SHA / ETag cache check
                                ▼
┌───────────────────────────────────────────────────────────────────────────┐
│              Browser Anki Engine (JSZip + Zstd + SQL.js + Proto)          │
│  ┌─────────────────────┐  ┌──────────────────────┐  ┌──────────────────┐  │
│  │  APKG Parser        │  │  FSRS-6 + Anki       │  │ Curriculum APKG  │  │
│  │  (anki21b & anki21) │  │  Queue Scheduler     │  │ Exporter &       │  │
│  │  + Template / Cloze │  │  (1-to-1 fields +    │  │ Scoped Importer  │  │
│  │  + Media Extractor  │  │   FSRS {s, d, dr})   │  │ (Hosted-only ID) │  │
│  └──────────┬──────────┘  └──────────┬───────────┘  └────────┬─────────┘  │
└─────────────┼────────────────────────┼───────────────────────┼────────────┘
              │                        │                       │
              ▼                        ▼                       ▼
┌───────────────────────────┐   ┌───────────────────────────────────────────┐
│ IndexedDB Local Store     │   │ Firestore (`users/{uid}/ankiDecks/...`)   │
│ - Parsed Hosted Notes     │   │ - User Card Scheduling States (per deck)  │
│ - Templates & CSS         │   │ - Per-Deck Anki Options & Daily Limits    │
│ - Audio / Image Blobs     │   │ - Protected by Owner-Only Security Rules  │
└─────────────┬─────────────┘   └──────────────────────┬────────────────────┘
              │                                        │
              └───────────────────┬────────────────────┘
                                  ▼
┌───────────────────────────────────────────────────────────────────────────┐
│                      Anki Flashcards Tab Workspace                        │
│  ┌─────────────┐  ┌──────────────────┐  ┌──────────────┐  ┌────────────┐  │
│  │ Deck Tree & │  │ Active Card      │  │ Searchable   │  │ Options &  │  │
│  │ Custom Study│  │ Reviewer         │  │ Card Browser │  │ Stats View │  │
│  │ (Subdecks)  │  │ (Cloze/Media/    │  │ (Anki Query  │  │ (Maturity/ │  │
│  │             │  │  1-4/Undo/Flags) │  │  Inspector)  │  │  Forecast) │  │
│  └─────────────┘  └──────────────────┘  └──────────────┘  └────────────┘  │
└───────────────────────────────────────────────────────────────────────────┘
```

### Data Model & State Mapping

- **Hosted Package & Note/Card Catalog (IndexedDB + In-Memory Store)**
  - `HostedDeckManifest`: Discovered `.apkg` entries (`packageId`, `filename`, `sha`, `downloadUrl`, `updatedAt`).
  - `AnkiNoteType`: Model definition (`id`, `name`, `isCloze`, `fields[]`, `templates[]` with `qfmt`/`afmt`, `css`).
  - `AnkiNote`: Note record (`guid`, `mid`, `tags[]`, `fields[]`, `csum`, `packageId`).
  - `AnkiDeck`: Hierarchical deck node (`id`, `fullName` using `::`, `parentName`, `packageId`, `confId`).
  - `AnkiCard`: Hosted card instance (`cardKey = "${guid}:${ord}"`, `nid`, `did`, `ord`, `guid`, `packageId`) combined with per-user scheduling state (`type`, `queue`, `due`, `dueAtMs`, `ivl`, `factor`, `reps`, `lapses`, `left`, `flags`, `suspended`, `buriedUntilDay`, `fsrs: { s, d, dr, lastReviewMs }`, `updatedAtMs`).
  - `AnkiReviewLog`: Chronological review entries (`id`, `cardKey`, `ease`, `ivl`, `lastIvl`, `factor`, `timeMs`, `type`) stored compactly alongside user progress for stats, undo, and `.apkg` export.

- **Firestore Per-User Persistence (`firebase-blueprint.json` + `firestore.rules`)**
  - Subcollection `users/{userId}/ankiDecks/{deckChunkId}`:
    - Stores `userId`, `packageId`, `chunkIndex`, `cards` map (`cardKey -> compact AnkiSchedulingPayload`), `deckOptions` map (`deckName -> AnkiDeckOptions`), `dailyUsage` (`date`, `newDoneByDeck`, `revDoneByDeck`), and `updatedAt`.
    - Strictly enforces `isEmailVerified()`, `isValidId(userId)`, `isValidId(deckChunkId)`, owner-only read/write (`request.auth.uid == userId`), `data.userId == userId`, bounded map sizes, and strict field type/length validation per the Eight Pillars and ESLint security checks.
  - User Root Document / Local Retention Sync:
    - Maintains lightweight `dueCardsCount` and daily quest progress (`srsReviewed`) when reviewing Anki flashcards in the workspace, while removing automatic FSRS card generation from quiz completions and removing old `QuizResults` `.apkg` export UI.

### Interactive Component & State Mapping

- **GitHub Curriculum Loader**: Fetches `.apkg` list from configured GitHub repo -> diffs file SHA against IndexedDB -> downloads & unpacks only new/changed `.apkg` archives -> caches media blobs -> merges with user's Firestore scheduling map -> notifies Deck Tree & Navbar due badge.
- **Deck Tree & Custom Study Panel**: Clicking a parent or nested `::` deck opens its study queue honoring `newPerDay`, `revPerDay`, sibling burying, and `New/Learning/Review` gather order; clicking **Custom Study** creates a temporary filtered study session (e.g., review forgotten cards, preview all cards, study by tag/flag).
- **Reviewer Engine**: Renders Mustache/Anki templates (`{{Field}}`, `{{cloze:Text}}`, `{{#Field}}...{{/Field}}`, `{{^Field}}...{{/Field}}`, `{{type:Field}}`, `{{FrontSide}}`, `{{Deck}}`, `{{Tags}}`) + resolves media references (`src` images to cached object URLs and `[sound:file]` tags to playable `<audio>` controls). Rating a card pushes an undo snapshot, advances FSRS-6 stability/difficulty + Anki queue/due/ivl/reps/lapses, updates daily counters/XP, persists locally immediately, and debounces/flushes batched writes to Firestore.
- **Scoped Anki Importer**: Accepts `.apkg`/`.colpkg` -> detects `collection.anki21b` (zstd + protobuf) vs `collection.anki21`/`collection.anki2` -> queries `notes` + `cards` (+ `revlog` fallback if `cards.data` FSRS fields aren't populated yet, reconstructing FSRS `s`/`d` from `revlog` or Anki `ivl`/`factor`) -> filters strictly against the hosted `cardKey` index -> previews matched vs skipped counts -> merges newer/chosen scheduling into Firestore & local store.
- **Curriculum Exporter**: Collects all hosted notes, templates, decks, media, `revlog`, and user scheduling states -> writes SQLite tables (`col`, `notes`, `cards`, `revlog`) with exact `due` encoding (collection-relative day index for review cards, epoch seconds for learning cards, queue/type codes `0..3`, negative queues `-1/-2` for suspended/buried, JSON `{"s":..., "d":..., "dr":...}` in `cards.data`) -> packages ZIP `.apkg` and triggers browser download.
