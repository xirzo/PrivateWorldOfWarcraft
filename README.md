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

Then open *WTF/Config.wtf* file in an text editor and replace *SET locale "enUS"* with *SET locale "ruRU"*.


## Adding game to Steam

Now go to *Steam* -> *Library* -> *Add a game* -> *Add a Non-Steam game* and put a path to *WoW.exe*. 

### Forcing game to use russian locale

Now open game's properties and insert this command to *Launch Options*:

```
LC_ALL=ru_RU.UTF-8 LANG=ru_RU.UTF-8 %command%
```

> [!NOTE]
> If you use *Linux* download [ProtonUp-Qt](https://github.com/DavidoTek/ProtonUp-Qt) and install latest *GE-Proton* with it. And force *WoW.exe* to use *GE-Proton* in *Steam* properties.

> [!NOTE]
> If you'd like to change server you connect to add a line this line `set realmlist  <server_address>` to `*Data/ruRU/realmlist.wtf*
