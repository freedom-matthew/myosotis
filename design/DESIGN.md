# myosotis — Design spec v2

A personal, local-first AI flashcard desktop app. Tauri 2 + Svelte 5 + TypeScript + Tailwind v4 + shadcn-svelte.

**Source of truth, in order:** this file → `theme.css` → `mockups/screens/*.dc.html` (exact values) → `mockups/screens-dark/` → `mockups/card-types/`.

## How to read the mockups

The `.dc.html` files are design-tool artboards, not app code: plain HTML with inline styles plus a little template syntax — `{{value}}` = data, `<sc-if value="…">` = conditional, `<sc-for list="…" as="x">` = loop; the `<script type="text/x-dc">` block at the bottom holds sample data and state.
**Copy values and copy text, not markup.** Rebuild with Svelte components and Tailwind classes using the tokens in `theme.css`.
Every spacing, font size and radius in the mockups is already on the Tailwind scale (audited), so `padding: 12px 20px` → `py-3 px-5`, `font-size: 14px` → `text-sm`, `border-radius: 14px` → `rounded-xl`.
Hex values map 1:1 to tokens (e.g. `#3E6FB0` = `primary`, `#66645F` = `muted-foreground`, `#E6E2D9` = `border`).

---

## 1. Principles

1. **Cards are the product; sources are temporary.** A source exists only to produce cards. Once its cards are checked, the full text is deleted. After distilling, where knowledge came from doesn't matter: sources are never named by the user and never shown on cards (§5).
2. **Calm.** One primary action per screen. Blue only for the primary action, progress, focus and "your selection".
3. **Keyboard-first.** Every action in Review, Inbox and Tidy up has a key, shown as a small mono key hint.
4. **Serif for what you learn, sans for the tool.** Card content, excerpts, page titles: Newsreader. UI: Geist. Numbers (intervals, counts, keys): Geist Mono.
5. **Desktop app, not a website.** Fixed shell, only content scrolls; chrome text not selectable; default cursor on buttons; nothing important is hover-only.
6. **No gamification.** No streaks, badges, confetti or guilt.
7. **Never destroy learning silently.** Merges and deletions of cards with review history always need confirmation.

## 2. Brand

No logo or icon yet (deliberately). The brand is the wordmark: **`myosotis`** — always lowercase, Newsreader **italic** (the font's true italic), 24px (`font-serif italic text-2xl leading-none`). It sits at the top of the sidebar. Window title: `myosotis`.

## 3. Tokens

All tokens live in `theme.css`. Highlights:

| Token | Light | Use |
|---|---|---|
| `background` | `#F6F4EF` | window ("paper") |
| `foreground` / `foreground-secondary` | `#1D1E22` / `#4A4945` | ink / inactive nav, secondary labels |
| `muted-foreground` | `#66645F` | captions, metadata (AA on paper and white) |
| `card` / `card-border` / `card-answer` | `#FFFFFF` / `#E8E4DB` / `#34353A` | flashcard, panels / its hairline / answer + excerpt text |
| `primary` / `primary-strong` | `#3E6FB0` / `#2C5590` | forget-me-not blue / link hover, badge text |
| `primary-soft` / `primary-tint` | `#EEF3FA` / `#DCE6F4` | selected row, suggested merge / badges |
| `secondary` | `#FBFAF7` | grade buttons, secondary buttons, excerpt boxes |
| `muted` | `#F4F1EA` | explanation box, status pills |
| `accent` | `#E5E1D8` | hover / active nav item |
| `highlight` | `#F6E3A6` | source passage highlight (Inbox) |
| `hook` / `hook-foreground` | `#FBF4DE` / `#6E5A1E` | memory hook box, "similar card" pill |
| `warn-soft` / `warn-foreground` | `#FBE6D4` / `#8A3D0E` | wrong answer (orange, never red vs green) |
| `border` / `input` | `#E6E2D9` / `#DDD8CE` | hairlines / field borders |
| `sidebar` | `#F0EDE6` | sidebar background |

**Dark mode** = the same design with the Dusk palette (`.dark` in `theme.css`; references in `mockups/screens-dark/`). Notable flips: primary becomes light blue with dark text on it; the sidebar's "New source" button flips to light with dark text; shadows get darker and stronger.

### Typography — Tailwind default sizes only

| Role | Class | px | Font |
|---|---|---|---|
| Today headline | `text-5xl` | 48 | serif 400, `leading-[1.1]` |
| Page titles, review question, "all done" | `text-3xl` | 30 | serif 400 |
| Wordmark, tile numbers, card-preview questions | `text-2xl` | 24 | serif italic (wordmark) / sans 500 / serif |
| Review answer | `text-xl` | 20 | serif, `leading-normal` |
| Inbox card question, preview answers | `text-lg` | 18 | serif |
| Reading text: source text, excerpts, sub-headlines, large buttons | `text-base` | 16 | serif or sans |
| UI default: buttons, inputs, nav, rows, body | `text-sm` | 14 | sans |
| Captions, labels, key hints, intervals | `text-xs` | 12 | sans / mono |

### Spacing — Tailwind default scale only (4px grid)

| Where | Classes |
|---|---|
| Page padding | `p-12` (Today: `pt-16`), dense screens `px-10 pt-9` |
| Between page sections | `gap-10` / `gap-7` |
| Sidebar | `w-54 px-3.5 py-5`, groups `gap-5`, items `gap-0.5` |
| Standard button / input (`h-9`) | `px-3 py-2` |
| Large button (CTA, Show answer) | `px-5 py-3` / `py-4` |
| Tiles, panels | `p-5` / `p-6` |
| List rows | `px-4 py-3` |
| Review card | `px-12 py-11`, inner `gap-6` |
| Key hint | `px-1.5 py-0.5 text-xs font-mono` |

### Radius

| Class | px | Use |
|---|---|---|
| `rounded-sm` | 6 | key hints, inline chips, text highlights, chart bars |
| `rounded-md` | 8 | nav items, ghost buttons, segmented-control items |
| `rounded-lg` | 10 | buttons, inputs, small boxes inside a card |
| `rounded-xl` | 14 | large buttons (CTA, Show answer, grades, answer options), list rows |
| `rounded-2xl` | 16 | panels, tiles, settings sections, drop zone |
| `rounded-card` | 20 | the flashcard |
| `rounded-full` | — | badges, status pills, progress bar |

**Shadows:** `shadow-card` (flashcard only), `shadow-primary` (primary CTA, Good button), `shadow-popover` (dialogs, command palette). Everything else is flat with 1px borders.
**Icons:** Lucide (`@lucide/svelte`), 16px, stroke 1.8.

## 4. App shell & navigation

- **Window:** min 960×640. Native decorations on Windows/Linux; on macOS an overlay title bar with a 28px drag region (`data-tauri-drag-region`).
- **Sidebar** (shadcn-svelte `Sidebar`), 216px, `bg-sidebar`, right border: wordmark → **+ New** (`sidebar-primary`, `⌘ N`; opens New source) → **Search** (opens command palette, `⌘K`) → **Today**, **Inbox** (+ `primary-tint` count badge), **Library** → spacer → **Settings**. Item: `px-3 py-2 rounded-md text-sm`; active = `bg-sidebar-accent font-medium`.
- **Command palette:** `Command` in `Dialog`, centred, 560px, `shadow-popover`. Groups: Actions, Cards, Imports. (Not drawn yet.)
- **Review takes over the whole window** — no sidebar. Esc returns to where you came from.
- **Narrow windows** (< 720px; future phone): sidebar collapses to an icon rail, later bottom tabs.

## 5. Data model & source lifecycle (fire-and-forget)

The **card** is the primary entity. A **source** is a temporary import job.

```
Card      id, type, fields (per type), fsrs_state, created_at, suspended
Origin    card_id, source_title, source_kind (text|md|pdf|docx), excerpt (1–3 sentences), imported_at
          → internal only: grounds AI Explain and Rewrite; never shown
Import    id, title, full_text, status (generating|in_inbox|done), created_at
          → full_text is deleted when status becomes done (default; Settings can keep it)
```

- **Sources are never named.** An import is labelled automatically while it's in the Inbox: an opened file's name, or else the text's first few words.
- **While a source is in the Inbox** its full text is kept: needed for passage highlighting and for "Rewrite" (AI regenerates a card from the original passage).
- **When the Inbox for that import is finished** (all cards accepted / skipped / deleted), delete `full_text`. Each accepted card keeps its `Origin` with a short excerpt, invisible to the user: it only grounds AI Explain and Rewrite in what was read.
- Only text inputs for now: paste, `.txt`, `.md`, `.pdf`, `.docx`.

### Duplicates

1. **At import (main defence).** Each new card is compared with existing cards (embeddings, e.g. a local model or the provider's embeddings API; vectors stored in SQLite, e.g. `sqlite-vec`). Above a similarity threshold the card enters the Inbox as **possible duplicate** with two actions:
   - **Skip** (default, `S`) — drop the new card.
   - **Keep both** (`K`) — treat as a normal new card.
2. **Tidy up (periodic).** On demand, AI clusters similar existing cards and proposes merges on the Tidy up screen. A merge keeps the **review history of the member with the most reviews**, uses the suggested (editable) wording. "Not duplicates" remembers the decision so the pair is not proposed again. Nothing changes without confirmation.

## 6. Screens

### Today — `screens/Today.dc.html`
Date eyebrow → headline `text-5xl` "37 cards are due today." → "About 9 minutes · 28 reviews and 9 new" → **Start review** (primary, `Space`) + link "Only reviews, skip new". When nothing is due and the collection is empty, just a line of text; no New source button (the sidebar has it). Three tiles, sharing the full width (the sidebar already counts the Inbox): **Next 7 days** (bars, today in `chart-1`), **Retention, 30 days**, **Keeps slipping** (focused session with Rewrite / Explain). **Recent imports** list (title, status such as "12 in Inbox" or "22 cards", relative date).
States: nothing due → "Nothing due today." + next due time. No API key → quiet banner linking to Settings.

### Review — `screens/Review.dc.html` (the core loop)
- Top bar: End session (`Esc`) · counter "12 of 37" (mono) + 3px progress bar · Undo (`Z`).
- Card: max-w 680px, `rounded-card`, `shadow-card`, `border-card-border`. Type line (12px muted; only for Concept and Describe) → question `text-3xl` serif. No source name.
- Before reveal: wide secondary **Show answer** (`Space`), max 360px.
- After reveal (`animate-reveal`): divider → answer `text-xl` → ghost actions **Explain `X`** (inline `muted` box with AI explanation), **Suspend `S`**, **Edit card `E`**.
- Grade bar (`animate-grades`): 4 buttons, max 560px, `rounded-xl`; label + "key · interval" (mono); **Good** = `primary` + `shadow-primary`. Intervals from the FSRS preview.
- Done state: "That is everything for now." + next due summary + **Back to Today**.

### New source — `screens/Import.dc.html`
Title + helper text; segmented **Paste text | Open file**. Left: Text textarea (serif 16, word count; label shows "· file name" after opening a file) or dashed drop zone. No title: nothing is named. Right panel: **How much to extract** (Essentials ~5 / Balanced ~15 / Thorough ~30), **Card types** (questions and concepts always; checkbox "Describe in your own words"), **Focus on** (optional), **Generate cards** (`⌘↵`) + "model · about N cards". Generating goes straight to the Inbox; cards stream in. **Stop** during generation returns to New source; cards made so far stay in the Inbox. File types are listed as ".txt, .md, .pdf, .docx".

### Inbox — `screens/Curate.dc.html`
- Header: the import's automatic label, status line ("2 accepted · 3 to check · 1 possible duplicate · still generating…"), **Accept all remaining** (`⇧A`).
- Left: source text (serif) with the focused card's passage in `highlight`; label "Source · kept until this Inbox is done".
- Right: card list — accepted (collapsed, blue check), pending (question + answer), focused (expanded, `primary` border + ring; **Accept `A`**, **Edit `E`**, **Rewrite `R`**, **Delete `D`**), **possible duplicate** (hook-coloured pill "Similar card already in your collection", new vs existing side by side with the existing card's origin and review count; **Skip `S`** primary, **Add source to existing `M`**, **Keep both `K`**), merged (collapsed: "Source added to ⟨existing question⟩"), skeleton while generating.
- `J/K` move focus; key-hint footer.

### Library — `screens/Library.dc.html`
- Title, "1,284 cards", **Tidy up** button with a count badge, **Add card `N`** (primary; also in `⌘K`). Add card is a dialog: Question | Concept | Describe switch with a one-line hint, two serif fields whose labels follow the type (Question/Answer, Term/Meaning, Question/Model answer), **Add card `⌘↵`** + **Done**; it stays open for the next card. Cards added by hand show "By hand" as their origin.
- Search (`/`, searches questions and answers) + filter chips: All, Due today, New, Low recall, Suspended.
- Table: Question · Recall now (bar + %, FSRS retrievability) · Next. Selected row: `primary-soft` + 3px inset `primary` bar.
- Detail panel: Question, Answer (serif textareas), meta (Reviews, Next review), Suspend, Delete.

### Tidy up — `screens/Tidy.dc.html`
Breadcrumb "Library" → title → "4 groups of similar cards found among 1,284. Nothing changes until you confirm." + **Check again**. One group open at a time: members side by side (question, answer, review count; "Keeps its review history" pill on the member with most reviews) → **Suggested merged card** (`primary-soft` box) → **Merge `M`** (primary), **Edit merged card `E`**, **Not duplicates `N`**. Other groups are collapsed rows; done groups collapse to a check row.

### Settings — `screens/Settings.dc.html`
Single 720px column of white sections; rows = label (+ helper) left, control right.
- **AI provider:** Provider, Model, API key (password, Test, "Connected"; stored in the system keychain, never in the DB).
- **Reviewing:** New cards per day only (a − / + stepper in steps of 5, the number also typeable). Desired retention stays at FSRS's 90% and isn't shown: the app is deliberately minimal, with as few knobs as possible.
- **Appearance:** Theme (Light | Dark | System), **Size** (Default | Larger | Largest: the whole window zooms 100 / 115 / 130%, for big screens; `⌘+ ⌘− ⌘0` anywhere), Card font (Serif | Sans), Reduce motion.
- **Data:** **Source text** (Delete when the Inbox is done — default | Keep the full text), **Check new cards for duplicates** (on), Collection folder, Backup and export, **Restore…** (dialog listing backups newest first with date, time and card count → confirm "Restore the backup from …?" · **Restore and restart**; the current collection is backed up first).

## 7. Card types — `mockups/card-types/`

Three types, deliberately. All share the review shell: origin line → question → (back) divider → answer → extras. The origin line names the type for Concept ("⇄ Concept, both ways round") and Describe ("In your own words").

| Type | Front → back | Grading | Mockup |
|---|---|---|---|
| `question` Question | question → short answer | manual 1–4 | Type-QA |
| `concept` Concept | term → meaning in one sentence; accepting it adds the reverse card (meaning → term) | manual 1–4; the two directions are never reviewed on the same day | Type-TwoWay |
| `describe` Describe | open why/how question + textarea "Your description" (`⌘↵` to check · uses Claude) → your words, covered ✓ / missing ○, verdict pill; **Model answer `M`** on request | AI-suggested | Type-Explain |

The AI makes all three at import; Describe sparingly (1–3 per source) and only when "Describe in your own words" is ticked. Each Describe check is one AI call. "Show answer without checking" skips the AI and grades manually.

**Suggested grade (Describe):** a `muted` pill ("Mostly there · suggested grade: Hard"); that grade button is highlighted, **Enter** accepts, **1–4** always override. The AI returns `{covered[], missing[], suggested_grade}`. Verdicts: Again "Not yet", Hard "Mostly there", Good "Got the main point", Easy "All there".

## 8. Keyboard map

| Context | Keys |
|---|---|
| Global | `⌘N` new source · `⌘K` command palette · `⌘,` settings · `⌘1/2/3` Today / Inbox / Library · `⌘+ ⌘− ⌘0` size |
| Today | `Space` start review |
| Review | `Space` show answer · `⌘↵` check a description · `1 2 3 4` grade · `Enter` accept suggested grade · `M` model answer · `Z` undo · `E` edit · `X` explain · `S` suspend · `Esc` end |
| Card inputs | `A–D` choose · `T / F` true/false · `Tab` skip typing and reveal · `⌘↵` submit an explanation |
| Inbox | `J / K` move · `A` accept · `E` edit · `R` rewrite · `D` delete · `⇧A` accept all · duplicates: `S` skip · `K` keep both |
| Tidy up | `M` merge · `E` edit merged card · `N` not duplicates · `J / K` next / previous group |
| Library | `/` search · `↑ ↓` move · `Enter` edit · `N` add card |
| New source | `⌘↵` generate |

`⌘` on macOS, `Ctrl` elsewhere. Single-letter keys are inactive while a text field has focus.

## 9. Motion

One easing: `--ease-calm` = `cubic-bezier(.2,.7,.2,1)`.

| What | How |
|---|---|
| Answer / explanation / context reveal | fade + 6px rise, 260ms (`animate-reveal` or Svelte `fly={{ y: 6, duration: 260 }}`) |
| Grade bar | same, 220ms, 40ms delay |
| Next card | question cross-fades, 180ms — no sliding or flipping |
| Progress bar | width 400ms |
| Hover / press | colour 120ms; press = 1px down |
| Panels (Library detail, Inbox focus, Tidy group) | fade + 6px slide, 220ms |
| Skeleton rows | shimmer 1.4s |

Reduce motion (setting or OS) makes every animation instant.

## 10. Copy

Sentence case. No exclamation marks, no "Oops". Short and specific ("Cards land in your Inbox for a quick check."). Data numbers in mono. Relative dates ("today", "3 days").

## 11. Not designed yet — build from the tokens

First run (welcome → AI provider and key → first source) · empty states (empty Inbox: "Nothing to check." only, empty Library, nothing due, Tidy up "All tidy." — both centred, nothing else on the page) · errors as inline `warn-soft` messages with Retry, never modal alerts · toasts (`Sonner`, bottom-centre: "Card deleted · Undo") · card edit dialog in Review · command palette · logo/app icon.

## 12. Component mapping (shadcn-svelte)

Conventions: everything clickable shows the pointer cursor and a visible focus ring. Segmented switches (ToggleGroup) are a tinted track (`bg-accent/70`, `p-1`, `rounded-lg`) with fully rounded items; the selected item is a white chip (`bg-card`, `shadow-xs`). Library filters are separate pills (`rounded-full`; selected = `bg-foreground`). Shortcut hints space their keys: "⌘ N", "⌘ ↵". Tables use 20px outer and 12px inner cell padding, 12px muted headers.


`Sidebar` · `Button` (default = primary, secondary, outline, ghost) · `Input` · `Textarea` · `Select` · `Checkbox` · `Slider` · `Switch` · `ToggleGroup` (segmented controls) · `Table` · `Badge` · `Command` + `Dialog` · `Tooltip` · `Separator` · `Sonner` · `Skeleton` · `Progress`.

Custom: `Flashcard` (one body per card type), `GradeBar`, `Kbd`, `ForecastBars`, `SourceText` (passage highlighting), `RecallBar`, `DuplicateCompare`, `MergeGroup`, `Wordmark`.
