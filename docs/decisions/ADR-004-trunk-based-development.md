# ADR-004: Trunk-Based Development

**Status:** Accepted
**Date:** 2026-07-06

## Context

The original branch strategy used `main` → `develop` → `feature/*`. For a solo developer or very small team, this introduces unnecessary overhead:
- Extra merge step from feature → develop → main.
- `develop` lags behind `main` or drifts out of sync.
- No meaningful quality gate between develop and main when one person owns both.

## Decision

Adopt trunk-based development:

```
main ─── feature/v0.1-foundation ─── feature/v0.2-agent ─── ...
```

- `main` — Always releasable. Production-ready.
- `feature/*` — Short-lived branches for each version milestone. Merged directly to `main` via PR.
- Tags — `v0.1.0`, `v0.2.0`, ... created on `main` after each merge.

No `develop` branch. No `release/*` branches. No long-lived feature branches.

## Rationale

- Fewer branches = less overhead for a small team.
- Trunk-based is the industry standard for continuous delivery.
- Merges are frequent and small (per-milestone).
- No risk of `develop` drifting from `main`.

## Consequences

- Feature branches must be short-lived (days to weeks, not months).
- Main must always be green (CI passes on every commit).
- Release tagging replaces branch-based release management.
- If the team grows beyond 2–3 people, reintroduce `develop` or adopt release branches.

## References

- [Trunk-Based Development](https://trunkbaseddevelopment.com/)
