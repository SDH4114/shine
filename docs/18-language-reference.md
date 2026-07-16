# Полный справочник языка

## Идентичность

- имя: Shine;
- расширение: `.shn`;
- CLI: `shine`;
- версия документации: 0.2.0;
- реализация: Rust stable;
- backend: tree-walking evaluator.

## Ключевые слова

| Слово | Назначение |
|---|---|
| `fn` | функция |
| `import`, `from`, `as` | импорт модуля или имени |
| `export` | открыть top-level имя другим модулям |
| `const` | константа |
| `if` | условие |
| `else` | альтернативная ветка |
| `loop` | единственный цикл |
| `in` | перебор или принадлежность |
| `step` | шаг диапазона |
| `return` | возврат из функции |
| `break` | выход из цикла |
| `continue` | следующая итерация |
| `true`, `false` | логические литералы |
| `none` | пустое значение |
| `and`, `or`, `not` | логические операторы |

## Типы аннотаций

```text
Int
Float
Number
String
Bool
List
List[Int]
List[Float]
List[Number]
List[String]
List[Bool]
List[List[Int]]
None
```

Вложенные `List[...]` parser принимает рекурсивно.

## Литералы

```text
42                  Int
3.14                Float
6.022e23            Float
8_000_000           Int
"text"              String
"""text"""          multiline String
true / false        Bool
none                None
[a, b, c]           List
```

## Инструкции

```shine
import math
import science.stats as stats
from math import square
from math import square as powerTwo
export fn calculate() { return square(2) }
```

```shine
name = expression
name: Type = expression
const name = expression
const name: Type = expression
name += expression
name -= expression
name *= expression
name /= expression
list[index] = expression
[a, b] = expression
```

```shine
fn name(param, typed: Type): ReturnType {
    return expression
}
```

```shine
if condition {
} else if condition {
} else {
}
```

```shine
loop {
}

loop condition {
}

loop item in iterable {
}

loop i in start..end step amount {
}
```

## Операторы

```text
+  -  *  /  //  %  **
==  !=  <  <=  >  >=
not  and  or  in
=  -=  *=  /=
..
```

## Postfix-выражения

```shine
function(arguments)
list.method(arguments)
value[index]
value[start..end]
value[..end]
value[start..]
```

## Встроенные константы

```text
PI  E  INF  NAN
```

## Глобальные функции

### Общие

```text
print(value...)
input(prompt?)
length(value)
type(value)
number(value)
string(value)
bool(value)
assert(condition, message?)
```

### Файлы

```text
readFile(path)
writeFile(path, text)
```

### Математика

```text
abs(x)
round(x, digits?)
floor(x)
ceil(x)
pow(base, exponent)
min(value, ...)
max(value, ...)
sum(list)
sqrt(x)
sin(x)
cos(x)
tan(x)
asin(x)
acos(x)
atan(x)
log(x)
log10(x)
log2(x)
```

## Методы List

```text
add(value, ...)
del(index)
remove(value)
have(value)
index(value)
len()
clear()
copy()
unique()
reverse()
sort()
sum()
min()
max()
mean()
```

## Области видимости

- предопределённая область: встроенные константы;
- верхний уровень программы: функции, константы и переменные файла;
- вызов функции: параметры и локальные переменные;
- блок: локальные объявления;
- итерация цикла: переменная перебора.

Поиск имени идёт от внутренней области к внешней. Обычное присваивание изменяет ближайшее существующее имя; если его нет, создаёт локальное динамическое имя.

## Лимиты runtime

- `Int`: `i64`;
- `Float`: `f64`;
- глубина вызовов: 1000;
- защитный лимит условного цикла: 10 000 000 итераций;
- диапазон и `step`: только `Int`;
- срез: только прямой, правая граница исключена.

## CLI

```text
shine new <project>
shine run <file.shn>
shine check <file.shn>
shine build <file.shn>
shine fmt <file.shn>
shine test [project]
shine help
shine version
```
