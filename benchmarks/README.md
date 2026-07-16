# Shine vs Python vs C# benchmark

This suite runs the same algorithms and workload sizes in all three languages:

- integer-heavy linear congruential loop;
- floating-point `sin`, `cos`, `sqrt`, and `log` loop;
- list allocation, 100,000-element sort, indexing, and checksum;
- two internal rounds to make process startup less dominant;
- output checksums to prevent dead-code elimination and verify equivalent work.

Run from the repository root:

```bash
python3 benchmarks/run_benchmarks.py
```

Quick smoke run:

```bash
python3 benchmarks/run_benchmarks.py --runs 1 --warmups 0
```

Reuse existing release builds:

```bash
python3 benchmarks/run_benchmarks.py --no-build
```

The runner reports best, median, mean, relative wall time, and peak RSS where `/usr/bin/time` supports it. It aborts if integer/list checksums differ or floating-point results differ by more than `1e-6`.

## Interpretation

Shine 0.1.3 still uses its tree-walking evaluator. C# uses the .NET 10 Release JIT, and Python uses the installed CPython interpreter. Therefore this measures the current end-to-end runtimes, not the planned LLVM AOT backend. Run on an idle machine, keep it connected to power, close heavy applications, and compare medians rather than one result.

Keep constants and formulas synchronized in:

- `benchmark.shn`;
- `benchmark.py`;
- `csharp/Program.cs`.
