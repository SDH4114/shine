#!/usr/bin/env python3
"""Build, validate, and time the same workload in Shine, Python, Rust, C++, and C#."""

from __future__ import annotations

import argparse
import platform
import re
import statistics
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BENCHMARKS = ROOT / "benchmarks"


@dataclass
class Measurement:
    wall_seconds: float
    peak_rss_bytes: int | None
    values: dict[str, float]


def command_output(command: list[str]) -> str:
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=True)
    return result.stdout.strip()


def build() -> None:
    subprocess.run(["cargo", "build", "--release"], cwd=ROOT, check=True)
    subprocess.run(
        [
            "rustc",
            "benchmarks/rust/benchmark.rs",
            "--edition=2021",
            "-C",
            "opt-level=3",
            "-C",
            "codegen-units=1",
            "-o",
            "target/release/rust-benchmark",
        ],
        cwd=ROOT,
        check=True,
    )
    subprocess.run(
        [
            "c++",
            "benchmarks/cpp/benchmark.cpp",
            "-std=c++17",
            "-O3",
            "-DNDEBUG",
            "-o",
            "target/release/cpp-benchmark",
        ],
        cwd=ROOT,
        check=True,
    )
    subprocess.run(
        ["dotnet", "build", "benchmarks/csharp/Benchmark.csproj", "-c", "Release", "--nologo"],
        cwd=ROOT,
        check=True,
    )


def parse_values(stdout: str) -> dict[str, float]:
    values: dict[str, float] = {}
    for line in stdout.splitlines():
        if "=" not in line:
            continue
        name, raw = line.split("=", 1)
        if name in {"integer", "float", "list"}:
            values[name] = float(raw)
    if set(values) != {"integer", "float", "list"}:
        raise RuntimeError(f"benchmark produced incomplete checksums:\n{stdout}")
    return values


def timed_command(command: list[str]) -> Measurement:
    time_tool = Path("/usr/bin/time")
    wrapped = command
    rss_pattern: re.Pattern[str] | None = None
    rss_multiplier = 1
    if time_tool.exists() and platform.system() == "Darwin":
        wrapped = [str(time_tool), "-l", *command]
        rss_pattern = re.compile(r"^\s*(\d+)\s+maximum resident set size$", re.MULTILINE)
    elif time_tool.exists() and platform.system() == "Linux":
        wrapped = [str(time_tool), "-v", *command]
        rss_pattern = re.compile(r"Maximum resident set size \(kbytes\):\s*(\d+)")
        rss_multiplier = 1024

    started = time.perf_counter()
    result = subprocess.run(wrapped, cwd=ROOT, text=True, capture_output=True, check=False)
    elapsed = time.perf_counter() - started
    tolerated_macos_time_error = (
        platform.system() == "Darwin"
        and result.returncode == 1
        and "sysctl kern.clockrate" in result.stderr
    )
    if result.returncode != 0 and not tolerated_macos_time_error:
        raise subprocess.CalledProcessError(
            result.returncode,
            wrapped,
            output=result.stdout,
            stderr=result.stderr,
        )
    peak_rss = None
    if rss_pattern is not None:
        match = rss_pattern.search(result.stderr)
        if match:
            peak_rss = int(match.group(1)) * rss_multiplier
    return Measurement(elapsed, peak_rss, parse_values(result.stdout))


def validate(reference: dict[str, float], candidate: dict[str, float], language: str) -> None:
    for key in ("integer", "list"):
        if candidate[key] != reference[key]:
            raise RuntimeError(
                f"{language} produced a different {key} checksum: "
                f"{candidate[key]:.0f} != {reference[key]:.0f}"
            )
    if abs(candidate["float"] - reference["float"]) > 1e-6:
        raise RuntimeError(
            f"{language} produced a different float checksum: "
            f"{candidate['float']:.6f} != {reference['float']:.6f}"
        )


def human_bytes(value: int | None) -> str:
    if value is None:
        return "n/a"
    return f"{value / 1024 / 1024:.1f} MiB"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=int, default=5, help="measured process runs per language")
    parser.add_argument("--warmups", type=int, default=1, help="unmeasured warmup runs per language")
    parser.add_argument("--no-build", action="store_true", help="reuse existing release builds")
    args = parser.parse_args()
    if args.runs < 1 or args.warmups < 0:
        parser.error("runs must be positive and warmups cannot be negative")

    if not args.no_build:
        build()

    rust_version = command_output(["rustc", "--version"]).split()[1]
    commands = {
        "Shine 0.1.3": [str(ROOT / "target/release/shine"), "run", "benchmarks/benchmark.shn"],
        f"Python {platform.python_version()}": [sys.executable, "benchmarks/benchmark.py"],
        f"Rust {rust_version} Release": [str(ROOT / "target/release/rust-benchmark")],
        "C++ Release": [str(ROOT / "target/release/cpp-benchmark")],
        "C# .NET 10 Release": [
            "dotnet",
            "benchmarks/csharp/bin/Release/net10.0/Benchmark.dll",
        ],
    }

    reference = parse_values(command_output(commands[next(iter(commands))]))
    results: dict[str, list[Measurement]] = {}
    for language, command in commands.items():
        print(f"Running {language}...", flush=True)
        for _ in range(args.warmups):
            warmup = timed_command(command)
            validate(reference, warmup.values, language)
        measurements = [timed_command(command) for _ in range(args.runs)]
        for measurement in measurements:
            validate(reference, measurement.values, language)
        results[language] = measurements

    medians = {
        language: statistics.median(item.wall_seconds for item in measurements)
        for language, measurements in results.items()
    }
    fastest = min(medians.values())
    print("\nResults (lower is better)")
    print(f"{'Language':<25} {'best':>9} {'median':>9} {'mean':>9} {'relative':>10} {'peak RSS':>12}")
    for language, measurements in sorted(results.items(), key=lambda item: medians[item[0]]):
        walls = [item.wall_seconds for item in measurements]
        rss_values = [item.peak_rss_bytes for item in measurements if item.peak_rss_bytes]
        rss = int(statistics.median(rss_values)) if rss_values else None
        print(
            f"{language:<25} {min(walls):>8.3f}s {statistics.median(walls):>8.3f}s "
            f"{statistics.mean(walls):>8.3f}s {statistics.median(walls) / fastest:>9.2f}x "
            f"{human_bytes(rss):>12}"
        )

    print("\nValidated checksums")
    print(f"integer={reference['integer']:.0f}")
    print(f"float={reference['float']:.6f}")
    print(f"list={reference['list']:.0f}")


if __name__ == "__main__":
    main()
