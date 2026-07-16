# Циклы и диапазоны

В Shine только одно ключевое слово цикла — `loop`. Оно покрывает четыре сценария.

## Бесконечный цикл

```shine
loop {
    command = input("> ")
    if command == "exit" {
        break
    }
}
```

Без `break` цикл продолжается бесконечно. Runtime имеет защитный предел для условного цикла, но бесконечный `loop {}` должен завершаться логикой программы.

## Цикл с условием

```shine
number = 0

loop number < 10 {
    print(number)
    number += 1
}
```

Условие проверяется перед каждой итерацией. Если оно сразу ложно, тело не выполнится ни разу.

Runtime останавливает условный цикл после 10 000 000 итераций с понятной ошибкой, защищая от случайной бесконечности.

## Перебор списка

```shine
names = ["Amin", "Murad", "Orxan"]

loop name in names {
    print(name)
}
```

Переменная `name` локальна для итерации. Можно также перебирать строку по Unicode-символам:

```shine
loop letter in "Shine" {
    print(letter)
}
```

## Диапазон

```shine
loop i in 0..10 {
    print(i)
}
```

Правая граница исключена. Значения: `0, 1, 2, ..., 9`.

Диапазоны в MVP содержат только `Int`.

## Шаг

```shine
loop i in 0..10 step 2 {
    print(i)
}
```

Результат: `0, 2, 4, 6, 8`.

Обратный проход:

```shine
loop i in 10..0 step -1 {
    print(i)
}
```

Результат: `10, 9, ..., 1`.

Шаг должен быть ненулевым `Int`. Дробные шаги пока не поддерживаются.

Если `step` не указан, Shine выбирает `1` для возрастающего диапазона и `-1` для убывающего.

## break

`break` немедленно завершает ближайший цикл:

```shine
loop value in values {
    if value < 0 {
        break
    }
    print(value)
}
```

## continue

`continue` пропускает остаток текущей итерации:

```shine
loop i in 0..10 {
    if i % 2 == 0 {
        continue
    }
    print(i)
}
```

Печатаются нечётные значения.

## Вложенные циклы

```shine
loop row in 0..3 {
    loop column in 0..3 {
        print("{row}, {column}")
    }
}
```

`break` и `continue` относятся к ближайшему окружающему циклу.

## Область видимости

Переменная перебора после цикла недоступна:

```shine
loop item in [1, 2, 3] {
    print(item)
}
```

Для накопления результата создайте переменную заранее:

```shine
total = 0
loop value in [10, 20, 30] {
    total += value
}
print(total)
```

## Полезные шаблоны

Создание списка:

```shine
squares = []
loop i in 0..10 {
    squares.add(i ** 2)
}
```

Поиск:

```shine
found = false
loop value in values {
    if value == target {
        found = true
        break
    }
}
```

Фильтрация:

```shine
positive = []
loop value in values {
    if value > 0 {
        positive.add(value)
    }
}
```
