# Форматирование и сборка

## Форматирование

```bash
shine fmt src/main.shn
```

Formatter:

- сначала проверяет, что файл синтаксически корректен;
- использует четыре пробела на уровень блока;
- удаляет пробелы по краям обычных строк кода;
- удаляет завершающие точки с запятой;
- сохраняет содержимое многострочных строк.

Пример до:

```shine
fn main() {
print("Hello");
if true {
print("World");
}
}
```

После:

```shine
fn main() {
    print("Hello")
    if true {
        print("World")
    }
}
```

Formatter изменяет файл на месте. Используйте систему контроля версий.

## Проверка перед сборкой

```bash
shine check src/main.shn
```

`shine build` также автоматически запускает checker и не создаёт artifact для неправильной программы.

## Сборка bundle

```bash
shine build src/main.shn
```

Для однофайлового `main.shn` создаются:

```text
target/shine/main
target/shine/main.shn
```

Первый файл — копия исполняемого Shine backend под именем программы. Второй — проверенный исходник. При запуске backend определяет своё имя и выполняет соседний `.shn`:

```bash
./target/shine/main
```

Для многомодульного entry создаются executable и каталог `main.shine-src`. Каталог сохраняет относительную структуру импортированных `.shn`-файлов и `.entry`; переносить нужно оба artifact.

## Перенос bundle

Переносите оба файла вместе:

```text
my-program/
├── main
└── main.shn
```

На целевой машине должны совпадать операционная система и архитектура с той, где собран исполняемый файл. Для Unix может понадобиться право выполнения:

```bash
chmod +x main
```

## Что означает compiled в MVP

Frontend Shine действительно выполняет стадии:

```text
source → lexer → tokens/spans → parser → AST → checker → backend
```

Текущий backend исполняет связанный HIR/AST через evaluator. `build` упаковывает backend и source bundle, но пока не переводит программу в машинный код.

Это сознательная архитектурная граница 0.1.3. Зафиксированный следующий production backend:

- typed MIR/SSA;
- LLVM IR;
- native AOT object generation and linking.

## Release-сборка самого Shine

```bash
cargo build --release
```

Бинарник языка:

```text
target/release/shine
```

Установка:

```bash
./install.sh
```

## Проверка качества реализации

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```
