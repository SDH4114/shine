# Списки

`List` — главная и единственная обязательная коллекция Shine MVP. Отдельных Tuple и Set пока нет.

## Создание

```shine
empty = []
numbers = [1, 2, 3]
names = ["Amin", "Murad"]
mixed = [1, "two", true, none]
```

Аннотации:

```shine
anything: List = []
names: List[String] = ["Amin", "Murad"]
values: List[Float] = [1.0, 2.5]
```

## Индексы

Индексация начинается с нуля:

```shine
names = ["Amin", "Murad", "Ali"]
first = names[0]
second = names[1]
```

Поддерживаются отрицательные индексы:

```shine
last = names[-1]
beforeLast = names[-2]
```

Выход за границы вызывает `Index Error`.

## Изменение элемента

```shine
names[0] = "Orxan"
```

Индекс должен уже существовать. Для добавления используется `add`.

## Срезы

Правая граница исключена:

```shine
numbers = [0, 1, 2, 3, 4, 5]

middle = numbers[1..4]
firstThree = numbers[..3]
fromThree = numbers[3..]
all = numbers[..]
```

Результаты:

```text
[1, 2, 3]
[0, 1, 2]
[3, 4, 5]
[0, 1, 2, 3, 4, 5]
```

Отрицательные границы считаются от конца. Срез создаёт новый список. В MVP поддерживаются только прямые срезы; начало не должно быть больше конца, и отдельного шага среза нет.

## Конкатенация и повторение

```shine
combined = [1, 2] + [3, 4]
repeated = [0, 1] * 3
```

Обе операции создают новый список.

## Деструктуризация

```shine
point = [10, 20]
[x, y] = point
```

Количество имён и элементов должно совпадать.

## Методы списка

### `add(value, ...)`

Добавляет одно или несколько значений в конец. Изменяет исходный список. Возвращает `none`.

```shine
names = []
names.add("Amin")
names.add("Murad", "Ali")
```

Без аргументов вызывает `Argument Error`.

### `del(index)`

Удаляет элемент по индексу и возвращает удалённое значение.

```shine
values = [10, 20, 30]
removed = values.del(1)
assert(removed == 20)
assert(values == [10, 30])
```

Поддерживает отрицательные индексы. Некорректный индекс вызывает `Index Error`.

### `remove(value)`

Удаляет первое равное значение. Возвращает `true`, если элемент найден, иначе `false`.

```shine
names = ["Amin", "Murad", "Amin"]
removed = names.remove("Amin")
assert(removed)
assert(names == ["Murad", "Amin"])
```

### `have(value)`

Проверяет наличие значения и возвращает `Bool`.

```shine
exists = names.have("Amin")
```

Эквивалентная форма:

```shine
exists = "Amin" in names
```

### `index(value)`

Возвращает индекс первого совпадения или `false`.

```shine
names = ["Amin", "Murad"]
firstIndex = names.index("Amin")
missing = names.index("Ali")
```

`firstIndex` равен целому `0`, а `missing` — логическому `false`. Эти значения различаются: `0 != false`.

### `len()`

Возвращает длину как `Int`:

```shine
count = names.len()
```

Глобальная форма: `length(names)`.

### `clear()`

Удаляет все элементы. Изменяет список и возвращает `none`.

```shine
values.clear()
assert(values == [])
```

### `copy()`

Создаёт независимую копию списка:

```shine
original = [1, 2, 3]
copied = original.copy()
copied.add(4)
assert(original == [1, 2, 3])
```

Копирование контейнера независимое. Вложенные изменяемые значения в обычных динамических списках могут сохранять разделяемую внутреннюю структуру; `const` создаёт замороженное содержимое.

### `unique()`

Возвращает новый список без повторов, сохраняя порядок первых вхождений:

```shine
values = [3, 1, 3, 2, 1]
uniqueValues = values.unique()
assert(uniqueValues == [3, 1, 2])
```

Исходный список не меняется.

### `reverse()`

Разворачивает исходный список на месте и возвращает `none`:

```shine
values = [1, 2, 3]
values.reverse()
assert(values == [3, 2, 1])
```

### `sort()`

Сортирует исходный список по возрастанию и возвращает `none`:

```shine
values = [3, 1, 2]
values.sort()
assert(values == [1, 2, 3])
```

Используйте однородные списки сравнимых значений: числа или строки. Для смешанных несравнимых типов полезный порядок не определён.

## Числовые агрегаты

### `sum()`

```shine
total = [10, 20, 30].sum()
```

Для списка `Int` возвращает `Int`, при наличии `Float` — `Float`. Пустой список возвращает `0`.

### `min()` и `max()`

```shine
minimum = values.min()
maximum = values.max()
```

Пустой список вызывает `Value Error`.

### `mean()`

```shine
average = [10, 20, 30].mean()
```

Всегда возвращает `Float`. Пустой список вызывает `Value Error`.

Все числовые агрегаты требуют только числовые элементы.

## Константные списки

```shine
const POINT = [10, 20]
[x, y] = POINT
```

Чтение, индексирование, срез, `have`, `index`, `len`, `copy`, `unique`, `sum`, `min`, `max` и `mean` разрешены. Мутирующие операции запрещены:

- присваивание `POINT[0] = ...`;
- `add`;
- `del`;
- `remove`;
- `clear`;
- `reverse`;
- `sort`.

## Таблица методов

| Метод | Меняет список | Результат |
|---|---:|---|
| `add(values...)` | да | `none` |
| `del(index)` | да | удалённое значение |
| `remove(value)` | да | `Bool` |
| `have(value)` | нет | `Bool` |
| `index(value)` | нет | `Int` или `false` |
| `len()` | нет | `Int` |
| `clear()` | да | `none` |
| `copy()` | нет | новый `List` |
| `unique()` | нет | новый `List` |
| `reverse()` | да | `none` |
| `sort()` | да | `none` |
| `sum()` | нет | `Int` или `Float` |
| `min()` | нет | минимальный элемент |
| `max()` | нет | максимальный элемент |
| `mean()` | нет | `Float` |
