# Простые классы и объекты

Shine использует небольшую Python-подобную модель объектов. Не нужно писать `public`, объявлять interface или повторять `self` в параметрах каждого метода.

```shine
class Counter {
    value = 0
    private secret = 7

    fn init(start) {
        self.value = start
    }

    fn add(amount) {
        self.value += amount
        return self.value
    }

    private fn hidden() {
        return self.secret
    }

    fn reveal() {
        return self.hidden()
    }
}

counter = Counter(10)
counter.add(5)
print(counter.value)
```

## Правила

- Поля и методы public по умолчанию.
- `init` автоматически вызывается при `Class(arguments)`.
- `self` доступен внутри каждого метода и не указывается в списке параметров.
- Поля можно читать, присваивать и менять через `+=`, `-=`, `*=`, `/=`.
- `private` разрешён только перед полем или методом.
- Private member доступен из методов того же класса и вызывает `Access Error` снаружи.
- `const object = Class()` запрещает изменение полей, включая изменение через метод.
- Inheritance, static members, properties, interfaces и abstract classes пока не поддерживаются.

Новые public-поля можно добавлять объекту обычным присваиванием. Это сохраняет динамичность и простоту модели.
