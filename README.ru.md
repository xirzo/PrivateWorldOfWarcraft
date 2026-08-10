## Скачивание клиента

Скачайте клиент с [ChromieCraft](https://chromiecraft.com/en/downloads/), вы можете скачать его из другого места, просто убедитесь, что он чистый. Затем распакуйте его в любую папку.

## Установка русской локализации

Установите русскую локализацию с [Google Drive](https://drive.google.com/drive/folders/1r9tMosDbh18qqRSa_qIyOdEk9RaFQhU5?usp=sharing) и распакуйте её содержимое в корневую папку игры.

Должно получиться примерно так:

```
.
├── Battle.net.dll
├── ...
├── Data
│   ├── ...
│   └── ruRU
├── ...
├── Wow.exe
└── WTF
```

Затем откройте файл *WTF/Config.wtf* в текстовом редакторе и замените *SET locale "enUS"* на *SET locale "ruRU"*. (Вы можете сделать файлы *WTF/Config.wtf* и *Data/ruRU/realmlist.wtf* доступными только для чтения, но сначала попробуйте без этого).


## Добавление игры в Steam

Теперь перейдите в *Steam* -> *Библиотека* -> *Добавить игру* -> *Добавить стороннюю игру* и укажите путь к *WoW.exe*. 

> [!WARNING]
> Если вы используете *Linux*, скачайте [ProtonUp-Qt](https://github.com/DavidoTek/ProtonUp-Qt) и установите последнюю версию *GE-Proton* через него. И принудительно запускайте *WoW.exe* через *GE-Proton* в свойствах *Steam*.

> [!WARNING]
> Если вы хотите сменить сервер, к которому подключаетесь, добавьте строку `set realmlist  <адрес_сервера>` в файл `Data/ruRU/realmlist.wtf`, если я дал вам этот гайд, попросите айпи у меня.

### Принудительное использование русской локализации в игре

Теперь откройте свойства игры и вставьте эту команду в *Параметры запуска*:

```
LC_ALL=ru_RU.UTF-8 LANG=ru_RU.UTF-8 %command%
```

## Console Port (Xbox Gamepad/Steamdeck)

Если вы хотите иметь поддержку геймпада в игре, используйте Console Port. Просто следуйте руководству из репозитория: https://github.com/leoaviana/ConsolePortLK

Я добавил `WoWpadX.exe -l "/Путь/К/WoW.exe"` в настройки запуска и `PROTON_REMOTE_DEBUG_CMD="/Абсолютный_путь/К_вашему_исполняемому_файлу_WoWPadX/здесь" %command%` в *WoW.exe*. Всё, кажется, работает (по большей части).

# Настройка Steam

Красивые баннеры/фоны/иконки можно найти здесь: https://www.steamgriddb.com/search/grids?term=World+of+warcraft

# Благодарности

- ConsolePort: https://github.com/leoaviana/ConsolePortLK
- ChromieCraft: https://chromiecraft.com/en/downloads
- steamgriddb: https://www.steamgriddb.com/search/grids?term=World+of+warcraft
