# Guided for clients

## Downloading client

Download client from [ChromieCraft](https://chromiecraft.com/en/downloads/),
you may download it from somewhere else, just make sure it's clean. Then extract it
somewhere.

## Installing russian locale

Install russian locale from [Google Drive](https://drive.google.com/drive/folders/1r9tMosDbh18qqRSa_qIyOdEk9RaFQhU5?usp=sharing) and extract it's contents to game root directory.

It should look something like:

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

Then open *WTF/Config.wtf* file in an text editor and replace *SET locale "enUS"* with *SET locale "ruRU"*. (You may want to make *WTF/Config.wtf* and *Data/ruRU/realmlist.wtf*) readonly, first try it without it)


## Adding game to Steam

Now go to *Steam* -> *Library* -> *Add a game* -> *Add a Non-Steam game* and put a path to *WoW.exe*. 

> [!NOTE]
> If you use *Linux* download [ProtonUp-Qt](https://github.com/DavidoTek/ProtonUp-Qt) and install latest *GE-Proton* with it. And force *WoW.exe* to use *GE-Proton* in *Steam* properties.

> [!NOTE]
> If you'd like to change server you connect to add a line this line `set realmlist  <server_address>` to `*Data/ruRU/realmlist.wtf*

### Forcing game to use russian locale

Now open game's properties and insert this command to *Launch Options*:

```
LC_ALL=ru_RU.UTF-8 LANG=ru_RU.UTF-8 %command%
```

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
