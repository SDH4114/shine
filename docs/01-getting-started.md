# Установка и быстрый старт

## Требования

Для сборки Shine нужен Rust stable. Рекомендуется Rust 1.80 или новее.

Проверить установку Rust:

```bash
rustc --version
cargo --version
```

Если команд нет, установите Rust с официального сайта [rustup.rs](https://rustup.rs/).

## Установка Shine

На macOS и Linux используйте установщик репозитория:

```bash
cd /путь/к/shine
chmod +x install.sh
./install.sh
```

Он выполняет две операции:

1. собирает release-версию командой `cargo install --path .`;
2. делает команду `shine` доступной через каталог из `PATH`.

Проверка:

```bash
shine version
```

Ожидаемый результат:

```text
Shine 0.2.0
```

Если команда была установлена Cargo, но терминал пишет `command not found`, выполните на Apple Silicon Mac:

```bash
ln -sf "$HOME/.cargo/bin/shine" /opt/homebrew/bin/shine
rehash
```

## Создание первого проекта

Команду можно запускать из любой папки, доступной для записи:

```bash
cd ~/Desktop
shine new hello-shine
cd hello-shine
```

Shine создаст:

```text
hello-shine/
├── shine.toml
├── src/
│   └── main.shn
└── tests/
```

Запустите программу:

```bash
shine run src/main.shn
```

Результат:

```text
Hello, Shine
```

## Первая программа вручную

Файлы Shine имеют расширение `.shn`. Создайте `hello.shn`:

```shine
fn main() {
    name = "Amin"
    print("Hello, {name}")
}
```

Запустите:

```bash
shine run hello.shn
```

Функция `main` является стандартной точкой входа. Сначала Shine обрабатывает объявления верхнего уровня, затем вызывает `main()` без аргументов.

## Проверка программы

До запуска можно проверить синтаксис, имена, фиксированные типы и константы:

```bash
shine check hello.shn
```

Успешная проверка выводит:

```text
Checked hello.shn successfully.
```

## Запуск без установки

Разработчики самого языка могут запускать CLI через Cargo:

```bash
cargo run -- run examples/hello.shn
cargo run -- check examples/statistics.shn
```

## Следующий шаг

Перейдите к [проектам и командам CLI](02-projects-and-cli.md), затем изучите [основы синтаксиса](03-syntax.md).
