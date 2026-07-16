# Условия

## Обычный if

```shine
if temperature > 30 {
    print("Hot")
}
```

Скобки вокруг условия не обязательны. Тело обязательно заключено в `{}`.

## if / else

```shine
if score >= 60 {
    print("Passed")
} else {
    print("Failed")
}
```

`else` может находиться на той же или следующей строке после закрывающей скобки.

## else if

```shine
if temperature > 100 {
    print("High")
} else if temperature < 0 {
    print("Low")
} else {
    print("Normal")
}
```

Проверки выполняются сверху вниз. Исполняется первая ветка с истинным условием.

## Сложные условия

```shine
if age >= 18 and country == "AZ" {
    print("Adult user from Azerbaijan")
}
```

Для сложных выражений полезны скобки:

```shine
if isAdmin or (isOwner and not blocked) {
    print("Access")
}
```

## Условия с принадлежностью

```shine
allowed = ["admin", "editor"]

if role in allowed {
    print("Can edit")
}
```

## Область видимости ветки

Новое имя внутри ветки локально:

```shine
if true {
    message = "local"
    print(message)
}
```

После блока `message` неизвестно. Если переменная уже существовала снаружи, присваивание изменит внешнюю переменную:

```shine
message = "before"

if true {
    message = "after"
}

print(message)
```

## if не является выражением

В MVP `if` — инструкция, поэтому нельзя присвоить результат блока напрямую:

```shine
label = "negative"
if value >= 0 {
    label = "non-negative"
}
```

Тернарного оператора пока нет.
