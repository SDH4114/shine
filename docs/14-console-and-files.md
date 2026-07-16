# Консоль и текстовые файлы

## Вывод

```shine
print("Hello, Shine")
print("Value:", 42)
print()
```

`print` принимает любое количество значений, разделяет их пробелами и завершает строку переводом строки.

Интерполяция обычно даёт более контролируемый результат:

```shine
name = "Amin"
score = 95
print("{name}: {score} points")
```

## Ввод

```shine
name = input("Your name: ")
print("Hello, {name}")
```

`input` всегда возвращает `String`. Для чисел выполните преобразование:

```shine
age = number(input("Your age: "))
print("Next year: {age + 1}")
```

Если пользователь введёт нечисловой текст, `number` создаст `Conversion Error`.

## Простой интерактивный цикл

```shine
fn main() {
    loop {
        command = input("> ")

        if command == "exit" {
            break
        } else if command == "hello" {
            print("Hello!")
        } else {
            print("Unknown command: {command}")
        }
    }
}
```

## Чтение файла

```shine
text = readFile("notes.txt")
print(text)
```

`readFile(path)` читает весь UTF-8-текст и возвращает `String`.

## Запись файла

```shine
report = "Total: 42\nStatus: ready\n"
writeFile("result.txt", report)
```

Файл создаётся или полностью перезаписывается. Функция возвращает `none`.

Родительская папка должна уже существовать:

```shine
writeFile("output/report.txt", text)
```

Если `output/` отсутствует, возникает `File Error`.

## Относительные пути

Пути считаются от текущей рабочей директории терминала:

```bash
cd /path/to/project
shine run src/main.shn
```

При таком запуске `readFile("data/input.txt")` читает `/path/to/project/data/input.txt`.

## Абсолютные пути

```shine
text = readFile("/Users/name/Documents/notes.txt")
```

Абсолютные пути работают, если процесс имеет необходимые права.

## Обработка ошибок

В MVP нет `try/catch`. Ошибка чтения или записи:

- показывает `File Error` со строкой исходника;
- объясняет системную причину;
- завершает программу кодом `1`.

Проверяйте существование и права файлов до запуска или организуйте структуру проекта заранее.

## Пример обработки данных

```shine
fn main() {
    source = readFile("data.txt")
    report = "Characters: {length(source)}\n"
    writeFile("report.txt", report)
    print("Report written")
}
```

## Ограничения MVP

- только чтение и полная перезапись текстовых файлов;
- нет append-режима;
- нет бинарных файлов;
- нет API каталогов;
- нет JSON/CSV parser в стандартной библиотеке;
- нет обработки исключений внутри программы.
