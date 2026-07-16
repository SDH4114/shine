# RFC 0002: Runtime ABI and object model

Status: accepted architecture, implementation pending.

Scalars use native value representation. Strings, dynamic collections, and objects use reference counting plus cycle collection. The user-facing object model stays small: public-by-default fields and methods, automatic `self`, `init`, and optional enforced `private`. Native class layout remains hidden behind a stable runtime handle.

The C ABI exposes opaque handles, explicit retain/release operations, typed error results, and versioned symbols. Async tasks cannot share ordinary mutable objects across worker threads; shared mutation requires `Shared`, synchronization primitives, atomics, or channels.
