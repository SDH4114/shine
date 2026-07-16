# Тестирование программ

Тесты Shine — обычные `.shn`-программы в папке `tests/`.

## Первый тест

Создайте `tests/math.shn`:

```shine
fn main() {
    assert(2 + 2 == 4)
    assert(sqrt(16) == 4.0)
}
```

Запустите из корня проекта:

```bash
shine test
```

Результат:

```text
pass tests/math.shn

1 test(s) passed.
```

## Сообщение assertion

```shine
fn main() {
    values = [10, 20, 30]
    assert(values.mean() == 20.0, "mean must be 20")
}
```

Если условие ложно, сообщение становится заголовком `Assertion Error`.

## Организация

```text
tests/
├── arithmetic.shn
├── functions.shn
├── lists.shn
└── regression_zero_index.shn
```

Команда запускает только `.shn`-файлы непосредственно в `tests/`. Вложенные каталоги в MVP не обходятся.

Файлы сортируются по имени, но каждый тест должен быть независимым и не полагаться на порядок.

## Тестирование функций

Поскольку модулей пока нет, тестовый файл содержит проверяемую функцию:

```shine
fn clamp(value, minimum, maximum) {
    if value < minimum {
        return minimum
    } else if value > maximum {
        return maximum
    }
    return value
}

fn main() {
    assert(clamp(-5, 0, 10) == 0)
    assert(clamp(5, 0, 10) == 5)
    assert(clamp(15, 0, 10) == 10)
}
```

В 0.2 imports тестового entry разрешаются относительно каталога `tests/`. Импорт project packages из `src/` будет добавлен вместе с полноценным project/package loader; пока общий helper следует хранить внутри namespace тестов.

## Проверка ошибок

`shine test` рассчитан на успешные программы. Для проверки того, что ошибочный код действительно отклоняется, используйте внешний shell или Rust integration tests самого компилятора.

```bash
shine check tests/invalid-example.shn
```

Ожидаемый сбой должен иметь ненулевой код завершения.

## Тесты с файлами

Пути считаются от рабочей папки, поэтому тест может использовать подготовленную fixture:

```text
tests/
├── fixtures/
│   └── input.txt
└── files.shn
```

Но `shine test` не обходит `fixtures/`, поэтому текстовый файл не будет принят за тест.

```shine
fn main() {
    text = readFile("tests/fixtures/input.txt")
    assert(length(text) > 0)
}
```

Не изменяйте общие файлы в независимых тестах.

## Что тестировать

- обычный сценарий;
- пустые списки и строки;
- индекс `0` и последний индекс;
- отрицательные значения;
- границы диапазонов;
- типы возвращаемых значений;
- `index`, который возвращает `0` или `false`;
- конвертации строк;
- условия остановки циклов.

## Тесты самого языка

Разработчики Shine запускают Rust-набор:

```bash
cargo test
```

Он проверяет lexer/parser/runtime, типы, `const`, циклы, списки, CLI, сборку, установочные сценарии и примеры документации.
