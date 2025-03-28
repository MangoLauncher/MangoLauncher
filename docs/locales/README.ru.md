# Mango Launcher

Современный лаунчер для Minecraft с поддержкой модов и оптимизацией производительности.

## Установка

### Скачивание

1. Скачайте последнюю версию лаунчера с [GitHub Releases](https://github.com/MangoLauncher/MangoLauncher/releases)
2. Распакуйте архив в удобное место

### Первый запуск

При первом запуске лаунчер создаст следующую структуру:

```
mangoenv/
├── .minecraft/          # Папка с игрой и версиями
├── java/               # Установленные версии Java
│   ├── java-8/        # Java 8 для старых версий
│   ├── java-17/       # Java 17 для новых версий
│   └── java-20/       # Java 20 для снапшотов
├── versions/           # Кэш версий и манифестов
├── profiles/           # Профили пользователей
└── launcher.log        # Лог лаунчера
```

### Требования

- **Операционная система:**
  - Windows 10/11
  - macOS 10.15+
  - Linux (с поддержкой glibc 2.31+)

- **Java:**
  - Лаунчер автоматически скачает нужную версию Java при необходимости
  - Поддерживаемые версии:
    - Java 8 (для версий до 1.16)
    - Java 17 (для версий 1.17+)
    - Java 20 (для некоторых снапшотов)

- **Место на диске:**
  - Минимум 1 ГБ для лаунчера и базовой версии
  - Рекомендуется 4+ ГБ для модов и нескольких версий

## Использование

### Основные функции

1. **Управление версиями:**
   - Tab для переключения между списками версий
   - Поддержка ванильных версий
   - Поддержка Forge и OptiFine (скоро)
   - История использованных версий

2. **Профили:**
   - Создание нескольких профилей
   - Настройка памяти и аргументов Java
   - Сохранение настроек для каждой версии

3. **Настройки:**
   - Выбор языка (Русский/English)
   - Настройка интерфейса
   - Управление Java-окружением

### Горячие клавиши

- `↑/↓` или `j/k` - Навигация по меню
- `Tab` - Переключение между элементами/списками версий
- `Enter` - Выбор
- `Esc` - Назад/Выход
- `L` - Смена языка

## Решение проблем

### Java не найдена

Если при запуске игры появляется ошибка о том, что Java не найдена:
1. Лаунчер автоматически предложит скачать нужную версию
2. Вы можете указать путь к уже установленной Java в настройках

### Проблемы с запуском

1. Проверьте лог `mangoenv/launcher.log`
2. Убедитесь, что у вас достаточно свободного места
3. Проверьте права доступа к папке `mangoenv`

## Разработка

### Сборка из исходников

```bash
git clone https://github.com/yourusername/mango-launcher.git
cd mango-launcher
cargo build --release
```

### Зависимости для разработки

- Rust 1.75+
- Cargo
- Git

## Лицензия

Этот проект распространяется под лицензией MIT - подробности в файле LICENSE. 