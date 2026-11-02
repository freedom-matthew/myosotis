# Myosotis

Personal AI flashcard app (Myosotis = forget-me-not). Built by me, for me, to really use daily. Inspired by Memly, which isn't available in my country. I don't like Anki.

Single user, local-only data, desktop (PC) only for now. Sources go in, flashcards come out; reviews are scheduled with FSRS. A phone or web version may come later: keep that possible, don't build for it.

## Concept
- "Zettelkasten for lazy people": consume sources freely, then extract what's worth retaining as flashcards.
- Import model is fire-and-forget: source in, cards out. No repository of sources is kept.

## Premises
- Fully mine if ever open-sourced: local database, LLM of the user's choice.
- Written in Rust (I'm learning Rust; my background is C#).
- The project is also a vehicle for learning to build software properly with AI.

## Tech stack
- Rust (Cargo workspace), Tauri 2 desktop shell.
- SQLite via sqlx (compile-time checked queries, migrations via sqlx-cli).
- UI: SvelteKit with static adapter (no SSR), Svelte 5, TypeScript, Tailwind, shadcn-svelte.
- Supporting crates: serde, reqwest (LLM calls), an FSRS crate, chrono/time, thiserror (core), anyhow (app), keyring (API key), tauri-specta (TS bindings).

## Architecture
```
core/      domain: cards, reviews, FSRS scheduling, import parsing. Pure Rust, no I/O.
           Defines traits (ports), e.g. CardRepo, CardGenerator.
storage/   SQLite adapter implementing core traits
ai/        LLM adapter implementing CardGenerator
app/       Tauri shell; features/<slice>/ = one vertical slice per use case
           (Tauri command + handler). Composition root lives here.
ui/        Svelte frontend
```

Rules:
- `core` never depends on Tauri, sqlx, reqwest or any I/O.
- UI talks to Rust only through Tauri commands (local IPC). No HTTP server.
- Organise by feature (vertical slices), not by technical layer.
- Accept some duplication between slices; extract shared code only once a pattern has repeated about three times.
- Store all times in UTC. API key lives in the OS keyring, never in SQLite or files.
- Don't add new dependencies without asking first.

## Code style
- Always annotate the type of every variable binding, even where it can be inferred: `let cards: Vec<Card> = ...` in Rust, `const cards: Promise<Card[]> = ...` in TypeScript. I want to read types in the code, not rely on IDE hints.
- Never annotate closure parameters (`|error| ...`, not `|error: RepoError| ...`); it makes them unreadable.

## Svelte
Svelte 5 with runes only (`$state`, `$derived`, `$effect`, `$props`). Never use Svelte 4 syntax (`export let`, `$:`, stores for local state).

## Roadmap
1. Walking skeleton: one card read from SQLite through core, shown in the Svelte UI, with one acceptance test. **← current**
2. Review: due cards and FSRS scheduling (pure domain logic, ideal for TDD).
3. Import: parse a source, send it to an LLM, get cards back. AI comes last so the core never depends on it.

## Workflow: ATDD
For every slice:
1. **Plan first.** Propose the slice design and wait for my approval.
2. **Write the acceptance tests** (Given/When/Then style, against core or the slice handler).
3. **Explain each test** in plain words: what behaviour it pins down and why. Wait for my review.
4. **Implement** until all tests pass. Keep the change limited to the slice. Simplest thing that passes, then refactor with tests green.
5. **Summarise** what changed and anything I should look at closely in the diff.

Once I've approved acceptance tests, never change, delete or weaken them to make them pass. If one looks wrong, stop and tell me why. Your own unit tests for internals go in `#[cfg(test)]` modules; mention them when you add them.

## How to work with me
- Experienced in C#/.NET, new to Rust, Svelte and Tauri. When code uses a non-obvious Rust concept (ownership, lifetimes, traits, macros), explain it in a sentence or two, ideally with a C# comparison.
- Help me think, don't think for me. On design decisions, ask for my approach first, then give feedback. I decide the domain model (e.g. what a `Card` is).
- Prefer small, reviewable steps. Be brief and direct.
- Write in British English.

## Commands
Run from the workspace root. `.env` points sqlx's compile-time checks at `dev.db` (gitignored).
- `cargo test --workspace`: all Rust tests
- `cargo tauri dev`: run the app in development
- `cargo sqlx migrate add --source app/migrations <name>` / `cargo sqlx migrate run --source app/migrations`: migrations
- `cargo sqlx prepare --workspace`: refresh the offline query cache in `.sqlx/` (commit it) after changing any query

## Open decisions
- Source parsing for import (text, Markdown, PDF, web pages).
- How the LLM is called (user's API key vs subscription).
