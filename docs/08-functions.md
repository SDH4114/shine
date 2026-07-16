# Функции

## Объявление

```shine
fn greet(name) {
    print("Hello, {name}")
}
```

Функции следует объявлять на верхнем уровне файла. Имя функции доступно во всей программе после разбора, поэтому порядок объявлений не важен.

## Вызов

```shine
fn greet(name) {
    print("Hello, {name}")
}

fn main() {
    greet("Amin")
}
```

Количество аргументов должно совпадать с количеством параметров.

## Возврат значения

```shine
fn add(a, b) {
    return a + b
}

fn main() {
    result = add(10, 20)
    print(result)
}
```

`return` немедленно завершает текущий вызов. Функция без `return` возвращает `none`.

```shine
fn show(value) {
    print(value)
}
```

## Типы параметров

```shine
fn addInts(a: Int, b: Int) {
    return a + b
}
```

Аннотация проверяется при каждом вызове:

```shine
addInts(10, 20)
addInts(10, "20")
```

Второй вызов вызывает `Type Error`.

## Тип результата

```shine
fn circleArea(radius: Float): Float {
    return PI * radius ** 2
}
```

Если функция объявила результат, каждое фактически возвращённое значение должно быть совместимо с ним.

```shine
fn label(value: Int): String {
    if value > 0 {
        return "positive"
    }
    return "other"
}
```

Статический checker проверяет известные типы `return`, а runtime проверяет реальное значение.

## Необязательные аннотации

Можно типизировать только часть сигнатуры:

```shine
fn repeat(text: String, count) {
    return text * count
}
```

Динамический параметр проверяется во время выполнения операции.

## Несколько результатов

Отдельных tuple и multiple return нет. Верните список:

```shine
fn minMax(numbers) {
    return [numbers.min(), numbers.max()]
}

fn main() {
    [minimum, maximum] = minMax([4, 8, 2, 10])
    print("{minimum}..{maximum}")
}
```

## Локальные переменные

Каждый вызов получает свою область видимости:

```shine
fn square(x) {
    result = x * x
    return result
}
```

`x` и `result` недоступны снаружи функции. Функция может читать константы и переменные верхнего уровня.

## Рекурсия

Рекурсивные функции поддерживаются:

```shine
fn factorial(n: Int): Int {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}
```

Runtime ограничивает глубину вызовов 1000. При превышении возникает `Runtime Error`.

## main

```shine
fn main() {
    print("Entry point")
}
```

`main` вызывается автоматически без аргументов. Поэтому у неё не должно быть обязательных параметров. Возвращаемое значение `main` игнорируется.

## Пока не поддерживается

- короткая форма `fn square(x) = x ** 2`;
- значения параметров по умолчанию;
- именованные аргументы;
- перегрузка функций;
- вложенные функции и замыкания;
- generics;
- отдельные модули и импорты.
