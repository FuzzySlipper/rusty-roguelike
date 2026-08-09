# Rusty Roguelike agent guidance

## Repository role

Rusty Roguelike is one concrete collapsed-party first-person roguelike. It owns
its rules, generated-floor admission, session orchestration, complete saves,
protocol, controls, and presentation. It is not a reusable RPG framework and
must never depend on Rusty D20.

Rusty Engine owns reusable host-neutral mechanisms. Rusty Procgen owns
deterministic dungeon generation and validation. Pin reviewed public revisions,
call their public surfaces directly, and do not add sibling path fallbacks or
copy their implementations into this repository.

## Architecture

Read [docs/design.md](docs/design.md) before changing authority, dependency
direction, persistence, protocol generation, generation admission, or the turn
model. Use [docs/agent-code-atlas.md](docs/agent-code-atlas.md) for path-level
ownership.

- Rust is the sole semantic and authoritative gameplay runtime.
- TypeScript owns browser presentation and transient input state. It does not
  evaluate gameplay, visibility, initiative, pathfinding, or targeting.
- The collapsed party occupies one Rust-owned grid square. A party activation
  moves the whole party or performs exactly one action.
- Hidden or non-participating enemies remain dormant until Rust admits them.
- Enemy attacks target the party square before Rust selects an affected member.
- Browser APIs go through platform ports; backend calls go through transport;
  application state goes through store.
- Cross-language DTOs are generated from Rust and strictly decoded at the
  browser boundary.

## Work and verification

Treat a dirty worktree as shared state. Preserve unrelated changes. Commit and
push each reviewable milestone directly to the current branch and record exact
SHAs in Den.

Run the narrowest check first, then:

```bash
./scripts/verify.sh
```

User-visible work requires a real Rust-served browser scenario and inspected
desktop/mobile artifacts. Update [docs/source-provenance.md](docs/source-provenance.md)
when donor or dependency pins change, and [docs/known-limitations.md](docs/known-limitations.md)
when an intentional phase boundary remains.

## Den Guidance Bootstrap

- Project ID: `rusty-roguelike`
- Resolve live guidance with the Den MCP `get_agent_guidance` tool before
  substantial work.
- Treat the resolved Den guidance packet and its referenced Den documents as
  the source of truth.
- If Den is unreachable, stop and tell the user which Den tool or command
  failed and what you were about to do. Do not reconstruct Den state from local files.