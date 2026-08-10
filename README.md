[Русская версия с установкой локализации](README.ru.md)

## Downloading client

Download client from [ChromieCraft](https://chromiecraft.com/en/downloads/),
you may download it from somewhere else, just make sure it's clean. Then extract it
somewhere.

## Adding game to Steam

Now go to *Steam* -> *Library* -> *Add a game* -> *Add a Non-Steam game* and put a path to *WoW.exe*. 

> [!NOTE]
> If you use *Linux* download [ProtonUp-Qt](https://github.com/DavidoTek/ProtonUp-Qt) and install latest *GE-Proton* with it. And force *WoW.exe* to use *GE-Proton* in *Steam* properties.

> [!NOTE]
> If you'd like to change server you connect to add a line this line `set realmlist <server_address>` to `*Data/enUS/realmlist.wtf*

## Console Port (Xbox Gamepad/Steamdeck)

If you want to have nice gamepad support for the game use this.

Generally just follow the guide from this repository: https://github.com/leoaviana/ConsolePortLK

I've added `WoWpadX.exe -l "/Path/To/WoW.exe"` to launch settings and `PROTON_REMOTE_DEBUG_CMD="/Absolute Path/To Your WoWPadX Executable/here" %command%` to *WoW.exe*. Everything seems to work (Mostly).

# Steam Customization

You may find nice banners/backgrounds/icons here: https://www.steamgriddb.com/search/grids?term=World+of+warcraft

# Credits

- ConsolePort: https://github.com/leoaviana/ConsolePortLK
- ChromieCraft: https://chromiecraft.com/en/downloads
- steamgriddb: https://www.steamgriddb.com/search/grids?term=World+of+warcraft
