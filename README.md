<p align="center">
  <img width="1200" height="776" alt="a" src="https://github.com/user-attachments/assets/4cdb1172-80bb-4fe8-8d43-429717f17b5e" />
</p>

[Русская версия с установкой локализации](README.ru.md)

[Server-side guide](README.server.md)

## Downloading client

Download client from [ChromieCraft](https://chromiecraft.com/en/downloads/),
you may download it from somewhere else, just make sure it's clean. Then extract it
somewhere.

## Adding game to Steam

Now go to *Steam* -> *Library* -> *Add a game* -> *Add a Non-Steam game* and put a path to *WoW.exe*. 

Once it's added, **launch the game from *Steam***: find *WoW* in your *Library* and click *Play*. Do not run *WoW.exe* directly from the file manager — *Steam* (and *Proton* on Linux) must be running for the game to work.

> [!NOTE]
> If you use *Linux* download [ProtonUp-Qt](https://github.com/DavidoTek/ProtonUp-Qt) and install latest *GE-Proton* with it. And force *WoW.exe* to use *GE-Proton* in *Steam* properties.

> [!TIP]
> The game connects to your **local** server at `127.0.0.1` by default. If you want to play on a real server, you **must** change it: open *Data/enUS/realmlist.wtf* in a text editor and make it contain exactly `set realmlist <server_address>`, replacing `<server_address>` with the server's IP address or hostname (e.g. `set realmlist 123.123.123.123`). Save the file, then launch the game from *Steam*.

Done!

## Console Port (Xbox Gamepad/Steamdeck) (OPTIONAL)

If you want to have nice gamepad support for the game use this.

Generally just follow the guide from this repository: https://github.com/leoaviana/ConsolePortLK

I've added `WoWpadX.exe -l "/Path/To/WoW.exe"` to launch settings and `PROTON_REMOTE_DEBUG_CMD="/Absolute Path/To Your WoWPadX Executable/here" %command%` to *WoW.exe*. Everything seems to work (Mostly).

# Steam Customization (OPTIONAL)

You may find nice banners/backgrounds/icons here: https://www.steamgriddb.com/search/grids?term=World+of+warcraft

# Credits

- ConsolePort: https://github.com/leoaviana/ConsolePortLK
- ChromieCraft: https://chromiecraft.com/en/downloads
- steamgriddb: https://www.steamgriddb.com/search/grids?term=World+of+warcraft
