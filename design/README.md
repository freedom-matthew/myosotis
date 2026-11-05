# myosotis — design handoff

Put this `design/` folder in the root of the app repo.

```
design/
  DESIGN.md          ← the spec: principles, brand, tokens, data model, every screen, card types, keys, motion
  theme.css          ← shadcn-svelte / Tailwind v4 theme (light + dark): paste into src/app.css
  mockups/
    screens/         ← Today, Review, New source (Import), Inbox (Curate), Library, Tidy up, Settings
    screens-dark/    ← Today and Review in dark mode (same design, Dusk palette)
    card-types/      ← front and back of the 9 card types
```

Everything in the mockups is on Tailwind's default spacing and type scales and on shadcn's radius tokens (audited), so values translate 1:1 into classes.

## Add this to the repo's CLAUDE.md

```md
## Design
- UI spec: design/DESIGN.md. Tokens: design/theme.css. Reference mockups: design/mockups/.
- Mockups are reference HTML: copy values and copy text, never markup. Build with shadcn-svelte + Tailwind tokens.
- Only theme tokens and Tailwind's default scales (bg-background, text-muted-foreground, text-sm, p-5, rounded-card…). No raw hex, no arbitrary values like p-[13px].
- Cards are the primary entity; source full text is temporary (DESIGN.md §5). Never delete or merge cards with review history without user confirmation.
- Don't add UI that the spec doesn't describe; ask first.
- Every interactive element needs a keyboard path and a visible focus ring.
```

## Build order (one prompt each)

1. "Set up the theme from design/theme.css, self-host Geist, Geist Mono and Newsreader (incl. italic), and build the app shell and sidebar from DESIGN.md §2 and §4."
2. "Set up the data model from DESIGN.md §5 (cards, origins, imports) in SQLite, with seed data from the mockups."
3. "Build the Review screen from DESIGN.md §6 and mockups/screens/Review.dc.html, with all keyboard shortcuts and the motion from §9." ← spend the most time here
4. "Build Today."
5. "Build New source and the Inbox, including streaming cards during generation and the source-text lifecycle."
6. "Add the duplicate check at import and the Inbox duplicate state."
7. "Build Library, then Tidy up, then Settings."
8. "Add card types `cloze` and `reverse` (DESIGN.md §7). Then dark mode."

After each step, ask Claude Code to screenshot the running app and compare it with the mockup.
If you also use **Export → Handoff to Claude Code** from the Claude Design canvas, keep this folder anyway: it holds the decisions that aren't visible on the canvas (data model, duplicate rules, keys, grading).
