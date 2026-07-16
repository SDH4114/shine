#!/usr/bin/env python3
"""Identical composite benchmark for Shine, Python, and C#."""

import math

ROUNDS = 2
INTEGER_ITERATIONS = 750_000
FLOAT_ITERATIONS = 150_000
LIST_SIZE = 100_000


def integer_work() -> int:
    state = 1
    checksum = 0
    for i in range(INTEGER_ITERATIONS):
        state = (state * 1_664_525 + 1_013_904_223 + i) % 2_147_483_647
        checksum = (checksum + state) % 9_223_372_036_854_775_000
    return checksum


def floating_work() -> float:
    checksum = 0.0
    for i in range(FLOAT_ITERATIONS):
        x = (i + 1) * 0.00001
        checksum += math.sin(x) * math.cos(x) + math.sqrt(x + 1.0) + math.log(x + 1.0)
    return checksum


def list_work() -> int:
    values: list[int] = []
    state = 7
    for i in range(LIST_SIZE):
        state = (state * 48_271 + i) % 2_147_483_647
        values.append(state)
    values.sort()
    middle = LIST_SIZE // 2
    return values[0] + values[middle] + values[LIST_SIZE - 1] + len(values)


def main() -> None:
    integer_checksum = 0
    floating_checksum = 0.0
    list_checksum = 0

    for round_index in range(ROUNDS):
        integer_checksum += integer_work() + round_index
        floating_checksum += floating_work()
        list_checksum += list_work()

    print(f"integer={integer_checksum}")
    print(f"float={floating_checksum:.6f}")
    print(f"list={list_checksum}")


if __name__ == "__main__":
    main()
