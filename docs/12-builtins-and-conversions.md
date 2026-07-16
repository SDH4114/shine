# Встроенные функции и преобразования

Встроенные функции доступны без импортов.

## Общая таблица

| Функция | Назначение | Результат |
|---|---|---|
| `print(values...)` | Вывод | `none` |
| `input(prompt?)` | Чтение строки | `String` |
| `length(value)` | Длина строки или списка | `Int` |
| `type(value)` | Имя runtime-типа | `String` |
| `number(value)` | Преобразование в число | `Int` или `Float` |
| `string(value)` | Текстовое представление | `String` |
| `bool(value)` | Проверка истинности | `Bool` |
| `readFile(path)` | Чтение текста | `String` |
| `writeFile(path, text)` | Запись текста | `none` |
| `assert(condition, message?)` | Проверка тестового условия | `none` |

Математические функции описаны отдельно: [Математика](13-mathematics.md).

## print

```shine
print("Hello")
print("Count:", 10)
print()
```

Несколько значений разделяются одним пробелом. В конце всегда добавляется новая строка. Вызов без аргументов печатает пустую строку.

Для управляемого форматирования предпочтительна интерполяция:

```shine
print("Count: {count}")
```

## input

```shine
name = input("Your name: ")
```

Prompt печатается без автоматического переноса. Результат всегда `String`; завершающий `\n` или `\r\n` удаляется.

Вызов без prompt:

```shine
line = input()
```

Допускается не более одного аргумента.

## length

```shine
print(length("Shine"))
print(length([10, 20, 30]))
```

Работает только со строкой и списком. Для списка также доступен `values.len()`.

## type

```shine
print(type(none))
print(type(true))
print(type(10))
print(type(3.14))
print(type("text"))
print(type([]))
```

Возможные runtime-имена: `None`, `Bool`, `Int`, `Float`, `String`, `List`, `Range`, `Function`.

## number

Поддерживаемые входы:

```shine
integer = number("42")
decimal = number("3.14")
one = number(true)
zero = number(false)
same = number(10)
```

- строка без дробной части преобразуется в `Int`;
- строка с точкой или экспонентой — в `Float`;
- `true` превращается в `1`, `false` — в `0`;
- число возвращается без изменения вида.

Неверная числовая строка вызывает `Conversion Error`:

```shine
value = number("not a number")
```

## string

```shine
text = string(42)
flag = string(true)
listText = string([1, 2, 3])
```

Функция принимает любое runtime-значение.

## bool

```shine
assert(bool(1))
assert(not bool(0))
assert(bool("text"))
assert(not bool(""))
assert(bool([1]))
assert(not bool([]))
```

Правила истинности полностью перечислены в разделе [Операторы](06-operators.md#истинность-значений).

## assert

```shine
assert(2 + 2 == 4)
assert(values.len() > 0, "values must not be empty")
```

Если условие ложно, возникает `Assertion Error` и программа завершается кодом `1`. Второй аргумент задаёт сообщение ошибки.

`assert` особенно полезен в файлах `tests/*.shn`, но доступен в любой программе.

## Ошибки аргументов

Вызов с неправильным количеством аргументов создаёт `Argument Error`:

```shine
length()
number("10", "20")
```

Для пользовательских функций количество аргументов проверяют и `shine check`, и runtime. Для некоторых встроенных функций окончательная проверка выполняется при запуске.
