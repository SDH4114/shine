# Математика и научные вычисления

Математические функции доступны без `import`.

## Константы

| Имя | Значение |
|---|---|
| `PI` | π |
| `TAU` | 2π |
| `E` | число Эйлера |
| `PHI` | золотое сечение |
| `INF` | положительная бесконечность |
| `NAN` | нечисловое значение IEEE 754 |

Они встроены как неизменяемые значения `Float`:

```shine
circumference = 2 * PI * radius
growth = E ** rate
```

Пользователь может явно объявить собственную локальную или верхнеуровневую константу с тем же именем, затенив встроенную:

```shine
const PI = 3.1415926535
```

## Базовые функции

### `abs(x)`

Возвращает абсолютное значение как `Float`.

```shine
distance = abs(-10)
```

### `round(x, digits?)`

```shine
whole = round(3.6)
twoDigits = round(3.14159, 2)
negativeDigits = round(1234.0, -2)
```

Без второго аргумента возвращает округлённый `Int`. С количеством знаков возвращает `Float`.

### `floor(x)` и `ceil(x)`

```shine
down = floor(3.9)
up = ceil(3.1)
```

Возвращают `Float`.

### `pow(base, exponent)`

```shine
value = pow(2, 8)
```

Эквивалентно `2 ** 8`. Возвращает `Float`.

## Агрегаты

Глобальные `min` и `max` принимают один или несколько сравнимых аргументов:

```shine
lowest = min(4, 8, 2, 10)
highest = max(4, 8, 2, 10)
```

`sum`, `product`, `mean`, `median`, `mode`, `variance` и `std` принимают один числовой список:

```shine
total = sum([1, 2, 3])
average = mean([1, 2, 3])
spread = std([1, 2, 3])
```

Эквивалентные методы:

```shine
values = [1, 2, 3]
total = values.sum()
lowest = values.min()
highest = values.max()
average = values.mean()
middle = values.median()
spread = values.std()
```

## Корень

```shine
root = sqrt(16)
```

Отрицательный аргумент вызывает `Math Error`, потому что комплексные числа в MVP не поддерживаются.

## Тригонометрия

Все углы задаются в радианах:

```shine
s = sin(PI / 2)
c = cos(0)
t = tan(PI / 4)
```

Обратные функции:

```shine
angle1 = asin(1)
angle2 = acos(0)
angle3 = atan(1)
```

`asin` и `acos` принимают значения реального диапазона от `-1` до `1`. Вне домена возникает `Math Error`.

Также без imports доступны:

```shine
angle = atan2(y, x)
h = hypot(3, 4)
hyperbolic = sinh(1) + cosh(1) + tanh(1)
inverse = asinh(1)
degreesValue = degrees(PI)
radiansValue = radians(180)
```

## Логарифмы

```shine
natural = log(E)
decimal = log10(1000)
binary = log2(1024)
```

- `log` — натуральный логарифм;
- `log10` — десятичный;
- `log2` — двоичный.

Отрицательный аргумент вызывает `Math Error`. Для нуля IEEE 754-результат текущего runtime равен `-INF`.

## Экспоненты и округление

```shine
growth = exp(2)
binaryGrowth = exp2(8)
root = cbrt(27)
whole = trunc(3.9)
fraction = fract(3.25)
```

## Целочисленная математика

```shine
divisor = gcd(54, 24)
multiple = lcm(6, 8)
ways = factorial(5)
```

`factorial` принимает `Int` от `0` до `20`, чтобы результат помещался в текущий `Int`.

## Ограничения и IEEE-проверки

```shine
limited = clamp(value, 0, 100)
direction = sign(value)
finite = isFinite(value)
infinite = isInfinite(value)
notNumber = isNan(value)
```

## Научная запись

```shine
speedOfLight = 3e8
avogadro = 6.022e23
electronCharge = 1.602176634e-19
```

Числа с экспонентой являются `Float`.

## Полный список математических функций

```text
abs  round  floor  ceil  trunc  fract  pow  sqrt  cbrt
min  max  sum  product  mean  median  mode  variance  std
sin  cos  tan  asin  acos  atan  atan2
sinh  cosh  tanh  asinh  acosh  atanh
log  log10  log2  exp  exp2  degrees  radians  hypot
clamp  sign  gcd  lcm  factorial  isNan  isInfinite  isFinite
```

## Ошибки математики

Shine выдаёт понятную диагностику для:

- деления на ноль;
- переполнения `Int`;
- отрицательного аргумента `sqrt`;
- значений вне реального домена `asin` и `acos`;
- некорректной степени в области действительных чисел;
- ненулевых требований к шагу диапазона.

Продвинутые алгоритмы линейной алгебры, оптимизации, интегрирования, sparse-вычислений и типы `Array`, `Matrix`, `Complex`, `Fraction` относятся к официальным scientific-модулям следующих версий.
