# RFC 0001: Native compiler pipeline

Status: accepted architecture, implementation pending.

Shine uses AST → HIR → typed MIR/SSA → LLVM IR → object → native linker. There is no production bytecode VM. Dynamic values use a tagged runtime representation; inferred stable scalar values are unboxed. The current evaluator remains a semantic oracle until every conformance test passes on LLVM AOT.

Debug builds preserve source maps and runtime checks. Release builds enable LLVM optimization, dead-code elimination, specialization, and static runtime linking. Incremental compilation caches module interfaces, MIR, and object files by content hash.
