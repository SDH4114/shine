# Модули и развитие языка

## Реализовано в Shine 0.2

Один `.shn`-файл является одним модулем. Imports разрешаются относительно каталога entry-файла:

```text
src/
├── main.shn
├── math.shn
└── science/
    └── stats.shn
```

```shine
// math.shn
export fn square(value: Int): Int {
    return value * value
}

fn internalHelper() {
    return 0
}
```

```shine
// main.shn
import math as numbers
from science.stats import mean

fn main() {
    print(numbers.square(12))
}
```

Поддерживаются формы:

```shine
import math
import science.stats as stats
from math import square
from math import square as powerTwo
```

Модуль `science.stats` ищется как `science/stats.shn`, затем как `science/stats/mod.shn`. Импортировать можно только объявления с `export`. Циклические imports, отсутствующие модули, private symbols и конфликты alias завершаются `Module Error` до выполнения программы.

Namespace import в 0.2 предназначен для вызова экспортированных функций (`math.square(2)`). Экспортированные значения можно импортировать формой `from math import PI`. Универсальный member access появится вместе с object model.

## Pipeline 0.2

```text
entry source
→ source manager
→ recursive module resolver
→ parsed module graph
→ HIR name isolation and import linking
→ existing checker
→ reference evaluator
```

Имена каждого dependency module изолируются во внутреннем namespace. Entry-module сохраняет пользовательские имена. Imports не превращают private declarations в глобальные names.

## Зафиксированный синтаксис следующих этапов

Следующие формы являются design contract, но parser 0.2 пока их не принимает:

```shine
public class Model: BaseModel, Predictable {
    private weights: Array[Float]
    protected name: String
    internal cache: Map[String, Object]

    public init(weights: Array[Float]) {
        self.weights = weights
    }
}
```

- одиночное наследование класса;
- несколько interfaces;
- `public`, `private`, `protected`, `internal` проверяются компилятором;
- reflection не обходит visibility.

```shine
fn parse(text: String): Result[Float, ParseError] {
    return number(text)?
}

try {
    value = parse(input())?
} catch ParseError as error {
    print(error.message)
} finally {
    print("done")
}
```

```shine
async fn load(url: String): DataFrame {
    return await http.get(url).dataFrame()
}

async fn main() {
    async with TaskGroup() as tasks {
        first = tasks.spawn(load(firstUrl))
        second = tasks.spawn(load(secondUrl))
    }
}
```

Async использует structured concurrency, cancellation propagation и channels. Detached task требует явного `spawnDetached`.

## Native architecture contract

Production compilation не вводит bytecode VM:

```text
AST → HIR → typed MIR/SSA → LLVM IR → object files → native linker
```

Reference evaluator сохраняется только для conformance и differential tests. Полные решения описаны в `docs/rfcs/`.
