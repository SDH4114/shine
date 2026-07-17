# Shine

Shine is a readable programming language growing toward native scientific, data, ML, CLI, and server applications. Programs use the `.shn` extension. Version 0.1.3 includes modules, HIR linking, broad built-in mathematics, inferred constants, and a simple Python-like object model. The runtime keeps the tree-walking evaluator as its semantic fallback and automatically uses a compact numeric VM for typed numeric functions and integer-list hot paths.

Полная русскоязычная документация: [docs/README.md](docs/README.md).

```shine
fn main() {
    values: List[Float] = [10.0, 20.0, 15.0, 40.0]
    print("Mean: {values.mean()}")
}
```

## Install

Rust 1.80 or newer is recommended. On macOS and Linux, use the installer so the
`shine` command is available from every directory:

```bash
git clone <repository-url> shine
cd shine
chmod +x install.sh
./install.sh
shine version
```

The installer runs `cargo install`, then links the executable into a writable
directory already present in `PATH` (for example `/opt/homebrew/bin`). If no
such directory exists, it uses `~/.local/bin` and updates the current shell's
startup file.

If Shine was already installed with `cargo install --path .` but Zsh reports
`command not found: shine`, repair the existing installation with:

```bash
ln -sf "$HOME/.cargo/bin/shine" /opt/homebrew/bin/shine
hash -r
shine version
```

For development without installation, replace `shine` with `cargo run --` in
the commands below.

## First project

```bash
shine new demo
cd demo
shine run src/main.shn
```

`shine new` resolves relative paths from the current directory, so the command
can create a project in any folder where you have write permission.

The generated project contains:

```text
demo/
├── shine.toml
├── src/
│   └── main.shn
└── tests/
```

## CLI

```text
shine new <project>     Create a project
shine run <file.shn>    Run a program
shine check <file.shn>  Check syntax, names, and fixed types
shine build <file.shn>  Build target/shine/<name> plus its source bundle
shine fmt <file.shn>    Format a source file in place
shine test [project]    Run every tests/*.shn file
shine help              Show command help
shine version           Show the version
```

Projects can now use multiple modules:

```shine
import math as numbers
from science.stats import mean

fn main() {
    print(numbers.square(12))
}
```

Imported declarations must be marked with `export`. Module paths are resolved from the entry file's directory. See [modules and language evolution](docs/21-modules-and-language-evolution.md).

`shine build` packages the current evaluator executable and either one neighboring `.shn` file or a `.shine-src` directory for a multi-module program. It is not optimized native compilation yet; the committed 1.0 direction is LLVM native AOT.

## Language tour

Variables are dynamic unless annotated. Constants cannot be reassigned or mutated.

```shine
value = 10
value = "now text"

age: Int = 16
age = 17

const point = [10, 20]
[x, y] = point
```

Functions can optionally type their parameters and return value:

```shine
fn circleArea(radius: Float): Float {
    return PI * radius ** 2
}
```

Shine has one loop keyword with four forms:

```shine
loop { }
loop condition { }
loop item in values { }
loop i in 0..10 step 2 { }
```

Lists support indexing, forward slicing, concatenation, repetition, and these methods:

```text
add  del  remove  have  index  len  clear  copy
unique  reverse  sort  sum  product  min  max
mean  median  mode  variance  std
```

Math functions such as `sqrt`, `sin`, `log`, `round`, `min`, `max`, and `sum` are available without imports, as are `PI`, `TAU`, `E`, `PHI`, `INF`, and `NAN`. Console and text-file built-ins include `print`, `input`, `readFile`, and `writeFile`.

Important everyday mathematics and statistics are always available without imports, including exponentials, trigonometry, hyperbolic functions, angle conversion, `gcd`, `lcm`, `factorial`, `mean`, `median`, `mode`, `variance`, and `std`. Advanced domains remain official scientific packages.

Constants infer their type from their only value:

```shine
const GRAVITY = 9.80665
typed: Float = 9.80665
dynamic = 9.80665
```

Classes use a small Python-like model: members are public by default, `private` is enforced, `self` is automatic, and `init` is the constructor.

```shine
class Counter {
    value = 0
    private secret = 7

    fn init(start) { self.value = start }
    fn add(amount) { self.value += amount }
}
```

Tests are ordinary programs using the built-in `assert` helper:

```shine
fn main() {
    assert(2 ** 8 == 256.0, "powers should work")
}
```

## Comments and integer division

Both comments and integer division use `//` in the master specification. To keep tokenization predictable in this MVP, a `//` at the beginning of a logical line is a comment; inside an expression it is integer division. Put comments on their own line.

```shine
// This is a comment.
result = 10 // 3
```

Semicolons are optional. Braces define blocks; indentation is formatting only. Strings support `{expression}` interpolation and triple-quoted multiline text.

## Development

```bash
cargo fmt --all --check
cargo test
cargo run -- run examples/statistics.shn
```

## Performance benchmark

The repository includes checksum-validated equivalent workloads for Shine, Python, Rust, C++, and C#:

```bash
python3 benchmarks/run_benchmarks.py
```

See [benchmarks/README.md](benchmarks/README.md) for methodology and interpretation. Current results measure the tree-walking Shine runtime, not the planned LLVM AOT backend.

The current implementation still excludes Web, GUI, async, package management, scientific arrays, inheritance, and LLVM code generation. Their architecture is documented in [shine-master-specification.md](shine-master-specification.md); roadmap items are not presented as implemented features.
