# RFC 0002: Runtime ABI and object model

Status: accepted architecture, implementation pending.

Scalars use native value representation. Strings, dynamic collections, and objects use reference counting plus cycle collection. Public class layout is hidden behind a stable runtime handle. Visibility is enforced during semantic resolution and reflective access.

The C ABI exposes opaque handles, explicit retain/release operations, typed error results, and versioned symbols. Async tasks cannot share ordinary mutable objects across worker threads; shared mutation requires `Shared`, synchronization primitives, atomics, or channels.
