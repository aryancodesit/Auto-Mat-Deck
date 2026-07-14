# TD-006: Pluggable ID generator

**Priority:** Low
**Target:** Before v1.0

## Description

The current `new_id()` function in `model.rs` generates IDs from a
nanosecond timestamp. While sufficient for v0.2, two IDs created in the
same nanosecond (possible across threads) could theoretically collide.

## Recommendation

Introduce an `IdGenerator` trait:

```rust
trait IdGenerator {
    fn generate(&self) -> String;
}
```

With implementations:
- **TimestampGenerator** — current approach (keep as default)
- **UUIDv7Generator** — time-sortable UUIDs (recommended for stable)
- **SnowflakeGenerator** — distributed, machine-specific
- **TestGenerator** — deterministic sequence for tests

## Expected effort

Small. Affects only `model.rs` and the `impl_id!` macro.
