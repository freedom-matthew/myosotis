# Myosotis

Personal AI flashcard app (Myosotis = forget-me-not). Built by me, for me, to really use daily. Inspired by Memly, which isn't available in my country. I don't like Anki.

## Concept
- "Zettelkasten for lazy people": consume sources freely, then extract what's worth retaining as flashcards.
- Import model is fire-and-forget: source in, cards out. No repository of sources is kept.
- Review on both PC and phone (phone likely the main one), so it's a web app.

## Premises
- Fully mine if ever open-sourced: local database, LLM of the user's choice.
- Written in Rust (I'm learning Rust; my background is C#).
- The project is also a vehicle for learning to build software properly with AI.

## Architecture
- Leaning towards Vertical Slice Architecture (new to me) rather than DDD + Clean Architecture.
- Each feature is one slice through endpoint, handler and storage, built and tested end to end before the next.
- Accept some duplication between slices; extract shared code only once a pattern has repeated about three times.

## Roadmap
1. Add card: walking skeleton, no AI. Proves web stack, database and test loop. **← current**
2. Review: due cards and scheduling (pure domain logic, ideal for TDD).
3. Import: send a source to an LLM, get cards back. AI comes last so the core never depends on it.

## Workflow: ATDD
- Acceptance Test-Driven Development. I write the acceptance tests myself; you write the implementation that makes them pass.
- Each slice starts with my acceptance test describing the behaviour end to end. That test is the spec.
- My acceptance tests live in `tests/` (Cargo integration tests against the crate's public API). Treat that folder as read-only.
- Never change, delete or weaken my tests to make them pass. If a test looks wrong or ambiguous, stop and tell me why.
- You may add your own unit tests for internals inside `src/` (`#[cfg(test)]` modules), and mention them when you do.
- Implement the simplest thing that passes, then refactor with the tests green.

## How to work with me
- Help me think, don't think for me. On design decisions, ask for my approach first, then give feedback. I decide the domain model (e.g. what a `Card` is).
- Prefer small, reviewable steps. Explain Rust idioms briefly when you use something I may not know.
- Be brief and direct.