# Строки и интерполяция

## Обычные строки

Строка заключается в двойные кавычки:

```shine
name = "Shine"
empty = ""
```

Строки хранят Unicode. Индексы и длина работают по символам, а не по отдельным байтам UTF-8.

```shine
text = "Salam"
print(text[0])
print(length(text))
```

## Управляющие последовательности

Поддерживаются:

| Последовательность | Значение |
|---|---|
| `\n` | новая строка |
| `\t` | табуляция |
| `\r` | возврат каретки |
| `\"` | двойная кавычка |
| `\\` | обратная косая черта |

```shine
message = "First line\nSecond line"
path = "folder\\file.txt"
quote = "He said: \"Hello\""
```

Для неизвестной escape-последовательности lexer сохраняет символ после `\`.

## Многострочные строки

```shine
text = """
First line.
Second line.
"""
```

Переносы внутри тройных кавычек входят в значение.

## Интерполяция

В фигурных скобках можно писать выражение Shine:

```shine
name = "Amin"
age = 16
print("Name: {name}, age next year: {age + 1}")
```

Допустимы вызовы функций, методы и индексы:

```shine
values = [10, 20, 30]
print("Count: {values.len()}")
print("Mean: {values.mean()}")
print("First: {values[0]}")
print("Root: {sqrt(16)}")
```

Checker проверяет имена и синтаксис внутри интерполяции.

## Литералы фигурных скобок

Удвоенная скобка выводит одну:

```shine
print("Use {{value}} for interpolation notation")
```

Результат:

```text
Use {value} for interpolation notation
```

## Конкатенация

```shine
first = "Shine"
second = "Language"
full = first + " " + second
```

Обе стороны `+` должны быть строками. Для числа используйте интерполяцию:

```shine
count = 5
message = "Count: {count}"
```

или преобразование:

```shine
message = "Count: " + string(count)
```

## Повторение

```shine
separator = "-" * 40
```

Количество повторов должно быть неотрицательным `Int`.

## Индексирование

```shine
word = "Shine"
first = word[0]
last = word[-1]
```

Результат индекса — строка из одного символа. Выход за границы вызывает `Index Error`.

## Срезы

```shine
word = "Shine"
print(word[0..3])
print(word[..2])
print(word[2..])
```

Правая граница исключена. Поддерживаются отрицательные границы и только прямые срезы.

## Длина

```shine
count = length("Shine")
```

У строки нет метода `.len()` в MVP; используйте глобальную `length`.

## Поиск подстроки

```shine
if "ine" in "Shine" {
    print("Found")
}
```

Поиск чувствителен к регистру.

## Преобразование в строку

```shine
print(string(42))
print(string(true))
print(string([1, 2, 3]))
print(string(none))
```

Списки форматируются как `[1, 2, 3]`, строковые элементы внутри списка показываются с кавычками.
