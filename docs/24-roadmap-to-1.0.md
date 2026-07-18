# Дорожная карта Shine 1.0

Этот документ фиксирует обязательный порядок развития после Shine 0.2.0. Реализованные возможности и планы всегда обозначаются отдельно: наличие пункта в roadmap не означает, что он уже доступен в CLI.

## Общие правила

- существующие `.shn`-программы 0.1.x не ломаются;
- до 1.0 изменения языка только расширяют синтаксис и семантику;
- отдельные `Tuple` и `Set` не добавляются: используются `const List`, деструктуризация и `unique()`;
- tree-walking evaluator остаётся эталоном семантики;
- numeric VM остаётся выборочным ускорителем до готовности native backend;
- production pipeline: HIR → typed MIR/SSA → LLVM → самостоятельный executable;
- GUI, browser API и полноценный web-framework не блокируют 1.0.

## 0.2 — Dictionary

Реализовано в 0.2.0: динамические и типизированные словари, скалярные ключи, порядок добавления, индексирование, методы, итерация, независимое от порядка равенство, глубокий `const`, безопасные циклические графы, HIR/modules, evaluator fallback, formatter, документация и Zed grammar. Полный контракт: [Словари](23-dictionaries.md).

## 0.3 — типы и ошибки

- выделить semantic resolver и type checker из evaluator;
- добавить пользовательские `enum`, generics и interfaces/traits;
- добавить `Option[T]`, `Result[T, E]`, пользовательские типы ошибок и `?`;
- добавить `try/catch/finally`, stack trace и source location между модулями;
- проверять аргументы, все return paths, недоступный код и use-before-declaration;
- сохранить динамические переменные и необязательные строгие аннотации;
- добавить pattern matching для `enum`, `Option` и `Result`.

## 0.4 — проекты, пакеты и стандартная библиотека

- превратить `shine.toml` в manifest с entry, версией, профилем сборки и dependencies;
- добавить `shine.lock` и воспроизводимое разрешение версий;
- реализовать `init`, `add`, `remove`, `update`, `build`, `run`, `test`, `fmt`, `lint`, `doc`, `bench`, `publish`;
- поддержать local, Git и registry dependencies;
- добавить std-модули CLI arguments, files, directories, path, environment, process, JSON, CSV, time, regex, HTTP, archives и logging;
- расширить test runner: groups, expected errors, filters, setup/cleanup, reports и coverage;
- заменить текстовый formatter на AST formatter.

## 0.5 — native compiler и runtime

- зафиксировать runtime ABI и представление dynamic values;
- использовать tracing GC для обычных объектов и циклических графов;
- выделить типизированные буферы больших числовых массивов;
- реализовать typed MIR/SSA, оптимизации, LLVM IR, object generation и native linking;
- разворачивать стабильные `Int`, `Float`, `Bool` без dynamic boxing;
- добавить debug/release profiles, source maps и incremental compilation;
- добавить C ABI/FFI;
- `shine build` создаёт standalone executable без `.shn` bundle;
- основные платформы: macOS Apple Silicon и Linux x86-64; Windows проходит conformance до 1.0.

## 0.6 — параллельность

- threads, channels, locks и atomic values;
- structured concurrency, `async/await`, cancellation и task groups;
- безопасные parallel loops и worker pools;
- проверка объектов и типов при передаче между потоками;
- sync и async варианты HTTP и файловых операций.

## 0.7 — данные и научные вычисления

- `Array[DType]` с shape, strides, views и broadcasting; `Vector` и `Matrix` являются размерностями `Array`;
- SIMD CPU backend, BLAS/LAPACK и многопоточные операции;
- `Complex`, безопасные dtype conversions и missing values;
- `Series` и `DataFrame` на Arrow-совместимой памяти;
- CSV, JSON, Parquet и Arrow;
- filter, select, group, aggregate, join, sort, window и lazy execution;
- SQL и внешние форматы как официальные packages;
- единый API CPU, Metal и CUDA.

## 0.8 — ML

- `Tensor` поверх общей scientific memory system;
- autograd, computation graph и освобождение временной памяти;
- layers/modules, optimizers, losses, metrics и data loaders;
- сохранение и загрузка моделей;
- CPU, Metal и CUDA execution;
- ONNX import/export;
- официальные packages для preprocessing, classical ML и neural API;
- один код модели работает на CPU или доступном GPU через `device`.

## 0.9 → 1.0 — инструменты и стабилизация

- LSP: completion, diagnostics, hover, references, rename и go-to-definition;
- debugger, breakpoints, stack/variables, profiler и memory profiler;
- API docs generator и package search;
- registry security, signatures и dependency checksums;
- fuzzing lexer/parser/checker/runtime;
- versioned specification и автоматические compatibility tests;
- заморозить публичный синтаксис и семантику перед 1.0.

## Обязательная проверка этапа

После каждого изменения запускаются:

- debug и release tests;
- Clippy без warnings, format check и `git diff --check`;
- все примеры через `shine check` и `shine run`;
- CLI smoke из произвольной директории;
- тесты `const`, типов, modules, cyclic values и diagnostics;
- differential tests evaluator, numeric VM и, после появления, LLVM backend;
- тяжёлые checksum-совместимые benchmarks Shine, Python, C#, C++ и Rust.

Для Dictionary дополнительно проверяются empty/nested dictionaries, mixed values, все scalar keys, `NAN`, `Int`/`Float` equality, removal, order, `const`, cycles, imports и typed mutation.

## Критерии готовности 1.0

- CLI-программа читает arguments, config, environment и files, запускает processes и обрабатывает errors;
- data-программа читает CSV/JSON/Parquet, соединяет и группирует данные, считает и сохраняет результат;
- ML-программа обучает и сохраняет модель, затем выполняет inference на CPU или GPU;
- проект воспроизводимо устанавливает dependencies, собирается и тестируется одной CLI-командой;
- `shine build --release` создаёт standalone native program;
- программы Shine 0.1.x работают без изменения исходников.
