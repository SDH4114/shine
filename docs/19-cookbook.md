# Практические рецепты

## Среднее числового списка

```shine
fn main() {
    values = [10, 20, 15, 40, 35]
    print("Count: {values.len()}")
    print("Mean: {values.mean()}")
}
```

## Минимум и максимум

```shine
fn minMax(values) {
    return [values.min(), values.max()]
}

fn main() {
    [minimum, maximum] = minMax([4, 8, 2, 10])
    print("Range: {minimum}..{maximum}")
}
```

## Удаление повторов

```shine
fn main() {
    raw = [1, 2, 2, 3, 3, 3]
    clean = raw.unique()
    print(clean)
}
```

## Фильтрация положительных значений

```shine
fn positiveOnly(values) {
    result = []
    loop value in values {
        if value > 0 {
            result.add(value)
        }
    }
    return result
}

fn main() {
    print(positiveOnly([-5, 2, 0, 7, -1]))
}
```

## Сумма диапазона

```shine
fn main() {
    total = 0
    loop i in 1..101 {
        total += i
    }
    print(total)
}
```

## Таблица квадратов

```shine
fn main() {
    loop i in 1..11 {
        print("{i}² = {i ** 2}")
    }
}
```

## Площадь кругов

```shine
fn circleArea(radius: Float): Float {
    return PI * radius ** 2
}

fn main() {
    radiuses: List[Float] = [1.0, 2.0, 3.5, 5.0]
    loop radius in radiuses {
        print("Radius: {radius}, Area: {circleArea(radius)}")
    }
}
```

## Нормализация значения

```shine
fn clamp(value: Number, minimum: Number, maximum: Number): Number {
    if value < minimum {
        return minimum
    } else if value > maximum {
        return maximum
    }
    return value
}
```

## Подсчёт совпадений

```shine
fn countValue(values, target) {
    count = 0
    loop value in values {
        if value == target {
            count += 1
        }
    }
    return count
}
```

## Простой калькулятор

```shine
fn main() {
    first = number(input("First number: "))
    operation = input("Operation (+, -, *, /): ")
    second = number(input("Second number: "))

    if operation == "+" {
        print(first + second)
    } else if operation == "-" {
        print(first - second)
    } else if operation == "*" {
        print(first * second)
    } else if operation == "/" {
        print(first / second)
    } else {
        print("Unknown operation")
    }
}
```

## Создание текстового отчёта

```shine
fn main() {
    values = [12.5, 18.0, 16.5, 20.0]
    report = """
Scientific report
Count: {values.len()}
Minimum: {values.min()}
Maximum: {values.max()}
Mean: {values.mean()}
"""
    writeFile("report.txt", report)
    print("Saved report.txt")
}
```

## Консольное меню

```shine
fn main() {
    loop {
        print("1. Hello")
        print("2. Exit")
        choice = input("Choose: ")

        if choice == "1" {
            print("Hello, Shine!")
        } else if choice == "2" {
            break
        } else {
            print("Unknown option")
        }
    }
}
```
