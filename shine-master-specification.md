# Shine — Master Specification and Implementation Prompt

## Purpose

This is the self-contained source of truth for Shine. Give it to an AI or developer with no prior context. When a detail is not fixed, choose the simplest predictable option useful for real small projects.

## Identity

- Name: Shine
- Extension: .shn
- implementation language: Rust
- current release line: 0.1.3 modules, built-in mathematics, inferred constants, and simple classes
- long-term focus: native scientific computing, data, ML, CLI, and servers
- production backend direction: LLVM native AOT, without a production bytecode VM

Shine is a simple, readable, practical compiled language for mathematics, science, data, and console applications.

Principles: simplicity over feature count; readable code for beginners; curly-brace blocks; optional semicolons; built-in basic operations; human-readable diagnostics; a complete small MVP before advanced features.

## MVP scope

Required: numbers, strings, booleans, none, lists, dynamic variables, fixed-type variables, constants, expressions, functions, return, if/else, one loop construct, ranges, step, list indexing and slicing, list methods, destructuring, console I/O, mathematics, conversions, text files, diagnostics, and CLI commands new/run/check/build/fmt/test.

Not implemented in 0.1.3: inheritance, async, package registry, LLVM code generation, scientific Array/DataFrame/Tensor, notebook, server runtime, WebAssembly, browser, GUI, or full LSP. These are roadmap layers, not current claims.

## Shine 1.0 architecture contract

Shine preserves dynamic-by-default variables and optional fixed annotations. Constants never carry a type annotation because their type is inferred from their single immutable value. Classes use a deliberately small Python-like model: public by default, optional enforced `private`, automatic `self`, `init`, fields, and methods. Structured `async/await` and `Result` plus `try/catch` remain planned layers.

Compilation follows source → AST → HIR → typed MIR/SSA → LLVM IR → object files → native linker. The evaluator remains a test oracle only. Scientific computing uses homogeneous Array/Tensor buffers and CPU/CUDA/Metal kernels. DataFrame uses Arrow-compatible columnar memory. The official ecosystem targets capability-equivalence with NumPy, Pandas, and SciPy through a simpler Shine API, without requiring Python at runtime.

## Syntax

Blocks use braces. Indentation is not grammar. Semicolons are optional. Comments use double slash. Strings support interpolation with braces.

~~~shine
fn main() {
    name = input("Your name: ")
    print("Hello, {name}")
}
~~~

Optional multiline strings:

~~~shine
text = """
Multiline text.
"""
~~~

Keywords: fn, const, if, else, loop, in, step, return, break, continue, true, false, none.

Literals:

~~~shine
42
19.99
3e8
1.5e-10
8_000_000
"Hello"
true
false
none
[1, 2, 3]
~~~

## Types and variables

Shine is dynamic by default. A dynamic variable can change value and type.

~~~shine
value = 10
value = 19.5
value = "Hello"
value = true
~~~

An explicit annotation fixes the type but allows reassignment.

~~~shine
age: Int = 16
age = 17
age = "17"     // Type Error
~~~

A const forbids reassignment and mutation of its contained value. Its type is always inferred; `const name: Type = value` is intentionally invalid because the binding has only one immutable value.

~~~shine
const age = 16
const name = "Shine"
const numbers = [1, 2, 3]
numbers.add(4) // error
~~~

Built-in types: Int, Float, Number, String, Bool, List, None. Number accepts Int and Float. List annotations are supported:

~~~shine
names: List[String] = []
values: List[Float] = [1.0, 2.5]
~~~

Dynamic lists may contain different values. There are no mandatory Tuple or Set types: use lists, const, destructuring, and unique.

## Operators

Arithmetic: plus, minus, multiply, divide, integer divide, remainder, and exponentiation.

~~~shine
a + b
a - b
a * b
a / b
a // b
a % b
a ** b
~~~

Slash division should return Float for numeric operands. Division by zero is a clear runtime error.

Comparisons: equality, inequality, less-than, less-or-equal, greater-than, greater-or-equal. Logic: not, and, or. False and zero are distinct values.

Membership has both equivalent forms:

~~~shine
if "Amin" in names {
    print("Found")
}

if names.have("Amin") {
    print("Found")
}
~~~

Lists can be concatenated with plus, repeated with multiply, and sliced. Slice right boundaries are excluded. Supported forms include numbers[1..4], numbers[..3], and numbers[3..].

## Conditions and functions

~~~shine
if temperature > 100 {
    print("High")
} else if temperature < 0 {
    print("Low")
} else {
    print("Normal")
}
~~~

Chained comparisons are desirable for mathematical readability; they may be implemented after the basic parser.

The canonical function form is:

~~~shine
fn name(attributes) {

}
~~~

~~~shine
fn greet(name) {
    print("Hello, {name}")
}

fn add(a, b) {
    return a + b
}

fn addInts(a: Int, b: Int): Int {
    return a + b
}
~~~

Argument and result annotations are optional but must be checked when present. A short form may be added later:

~~~shine
fn square(x) = x ** 2
~~~

Main is the console entry point.

## One loop construct

There are no separate for and while statements. Only loop exists.

~~~shine
loop {
    command = input("> ")
    if command == "exit" {
        break
    }
}

number = 0
loop number < 10 {
    print(number)
    number += 1
}

names = ["Amin", "Murad", "Orxan"]
loop name in names {
    print(name)
}

loop i in 0..10 {
    print(i)
}

loop i in 0..10 step 2 {
    print(i)
}

loop i in 10..0 step -1 {
    print(i)
}
~~~

The range 0..10 means 0 through 9. MVP ranges are integers. Fractional steps can be added later. Support break and continue.

## Lists

List is the main and only required collection.

~~~shine
names = []
names = ["Amin", "Murad"]
first = names[0]
names[0] = "Ali"
~~~

Required methods:

- add(value): add one or more values
- del(index): remove by index and return the removed value
- remove(value): remove the first match and return Bool
- have(value): return Bool
- index(value): return index or false
- len(): return length
- clear(): empty the list
- copy(): independent copy
- unique(): new list with duplicates removed
- reverse(): reverse the list
- sort(): ascending sort

Numeric lists also support sum, min, max, and mean. Later add median, mode, variance, and std.

~~~shine
names.add("Amin")
removed = names.del(0)
exists = names.have("Amin")
index = names.index("Amin")
~~~

If index is absent, index returns false, not minus one and not none. Index zero must remain distinguishable from false.

Unique replaces the basic Set use case:

~~~shine
numbers = [1, 2, 2, 3, 3]
uniqueNumbers = numbers.unique()
~~~

A const list replaces the basic Tuple use case:

~~~shine
const point = [10, 20]
[x, y] = point
~~~

Multiple return values are ordinary lists:

~~~shine
fn minMax(numbers) {
    return [numbers.min(), numbers.max()]
}

[minimum, maximum] = minMax([4, 8, 2, 10])
~~~

## Mathematics and science

Math functions are built in without imports:

~~~shine
abs(-10)
round(3.14159, 2)
floor(3.9)
ceil(3.1)
pow(2, 8)
min(1, 2, 3)
max(1, 2, 3)
sum([1, 2, 3])
sqrt(x)
sin(x)
cos(x)
tan(x)
asin(x)
acos(x)
atan(x)
log(x)
log10(x)
log2(x)
exp(x)
exp2(x)
cbrt(x)
sinh(x)
cosh(x)
tanh(x)
atan2(y, x)
degrees(x)
radians(x)
hypot(x, y)
clamp(x, minimum, maximum)
gcd(a, b)
lcm(a, b)
factorial(x)
mean(list)
median(list)
mode(list)
variance(list)
std(list)
~~~

Built-in immutable constants: PI, TAU, E, PHI, INF, NAN.

Support scientific notation and digit separators:

~~~shine
speed = 3e8
avogadro = 6.022e23
population = 8_000_000_000
~~~

## Built-ins and files

Minimum global built-ins:

~~~shine
print(value)
input(prompt)
length(value)
type(value)
number(value)
string(value)
bool(value)
~~~

Text files:

~~~shine
text = readFile("notes.txt")
writeFile("result.txt", text)
~~~

File failures must produce a useful diagnostic and non-zero exit code.

## Diagnostics

Each error should contain a category, file, line, column, source line, pointer, explanation, and short suggestion.

~~~text
Type Error: variable age must contain Int

main.shn:4:1
4 | age = "seventeen"
    ^^^

The variable was declared as Int and cannot receive a String.
~~~

Const mutation must be diagnosed similarly. Errors go to stderr. Normal source errors must never surface as unexplained Rust panics.

## CLI and projects

The executable is shine.

~~~bash
shine new my-project
shine run main.shn
shine check main.shn
shine build main.shn
shine fmt main.shn
shine test
shine help
shine version
~~~

New projects:

~~~text
my-project/
├── shine.toml
├── src/
│   └── main.shn
└── tests/
~~~

Configuration:

~~~toml
[project]
name = "scientific-tool"
version = "0.1.0"
entry = "src/main.shn"
~~~

The first implementation may accept one file, but its APIs should be ready for multiple source files.

## Rust MVP architecture

Use Rust stable. Recommended libraries are clap for CLI, miette or ariadne for diagnostics, thiserror for internal errors, and serde/toml for project configuration. Keep dependencies small.

Suggested layout:

~~~text
shine/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── source.rs
│   ├── token.rs
│   ├── lexer.rs
│   ├── ast.rs
│   ├── parser.rs
│   ├── types.rs
│   ├── checker.rs
│   ├── value.rs
│   ├── env.rs
│   ├── evaluator.rs
│   ├── builtins.rs
│   ├── formatter.rs
│   ├── project.rs
│   └── diagnostics.rs
├── examples/
└── tests/
~~~

Pipeline:

~~~text
.shn source -> lexer -> tokens with spans -> parser -> AST
-> name and fixed-type checks -> evaluator/backend -> result or diagnostic
~~~

Keep AST nodes separate from runtime values. Minimum AST: Program, Block, declarations, assignment, functions, calls, return, if, the four loop forms, break, continue, literals, identifiers, unary/binary expressions, lists, indexing, index assignment, member calls, and destructuring.

Possible runtime representation:

~~~rust
enum Value {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(ListValue),
    Function(FunctionValue),
}
~~~

Each binding stores name, value, optional declared type, and const flag. Dynamic bindings accept supported values; typed bindings enforce their type; const forbids assignment and list mutation; unknown names produce source-based diagnostics.

Create a replaceable backend boundary:

~~~rust
trait Backend {
    fn execute(&mut self, program: &Program) -> Result<Value, Diagnostic>;
}
~~~

The first backend is a tree-walking Rust evaluator. It remains the semantic reference while HIR and LLVM native AOT are implemented. Do not add a production bytecode VM and do not claim optimized native compilation before LLVM object generation and linking exist.

## Real programs required for MVP

~~~shine
fn main() {
    print("Hello, Shine")
}
~~~

~~~shine
fn main() {
    values: List[Float] = [10.0, 20.0, 15.0, 40.0, 35.0]
    print("Count: {values.len()}")
    print("Sum: {values.sum()}")
    print("Mean: {values.mean()}")
    print("Minimum: {values.min()}")
    print("Maximum: {values.max()}")
}
~~~

~~~shine
fn main() {
    numbers = []
    loop i in 0..10 step 2 {
        numbers.add(i)
    }
    print(numbers)
}
~~~

~~~shine
const PI = 3.1415926535

fn circleArea(radius: Float): Float {
    return PI * radius ** 2
}

fn main() {
    radiuses: List[Float] = [1.0, 2.0, 3.5, 5.0]
    loop radius in radiuses {
        print("Radius: {radius}, Area: {circleArea(radius)}")
    }
}
~~~

## Development order

1. Cargo skeleton, CLI, file loading, help, version, and error format.
2. Lexer with source spans.
3. Parser and AST.
4. Evaluator, scopes, functions, loops, variables, types, const, arithmetic, and lists.
5. Built-ins, math, strings, conversions, and files.
6. Project creation, config, formatter, tests, and multi-file loading if practical.
7. Typed MIR/SSA, LLVM IR, object generation, and native AOT linking.

Every stage requires tests and a real CLI smoke test.

## Definition of done

The MVP is ready when:

1. shine new demo creates a runnable project.
2. shine run executes real .shn examples.
3. shine check catches syntax errors, unknown names, fixed-type violations, and const mutation.
4. All four loop forms work.
5. 0..10 yields 0 through 9 and step works.
6. Required list methods and numeric aggregates work.
7. index returns an index or false, including correct index zero handling.
8. Math functions work without imports.
9. Diagnostics show file, location, and a human explanation.
10. Unit, integration, and end-to-end tests exist.
11. Installation and first-run documentation exists.

## Future roadmap

OOP and visibility; typed errors; LLVM native AOT; async/server runtime; Array CPU/CUDA/Metal; Arrow DataFrame; science/ML packages; package manager; LSP; debugger; notebook; WebAssembly and other platforms after 1.0 foundations.

Future features must preserve the simple core.

## Prompt for the implementing AI

You are implementing Shine, a Rust-based language for mathematics, science, data, and console applications. This document is the source of truth.

Implement incrementally in the accepted roadmap order. OOP, async, scientific types, and native AOT are explicitly requested, but each layer must land only after its frontend/runtime contracts and regression tests are stable.

Before every change:

1. Inspect the repository and existing tests.
2. Identify the roadmap stage affected.
3. Preserve all fixed syntax and semantic rules.
4. Add or update tests.
5. Run real CLI commands, not only cargo check.
6. If a detail is unspecified, choose the simplest useful behavior and document it.

Prefer small, verifiable changes, stable diagnostics, and end-to-end behavior. Do not rewrite large areas unnecessarily.

Primary quality question:

> Can a beginner install Shine, create a project, write a small mathematical or console program in .shn, receive a clear error when code is wrong, and run the program successfully?

If not, fix that before adding more features.
