# How To

This script is used to offload building the server to a better PC, as I don't wanna do what on a steamdeck.

It just clones [AzerothCore](https://github.com/mod-playerbots/azerothcore-wotlk.git) and [mod-player-bots](https://github.com/mod-playerbots/mod-playerbots.git), then generates a `docker-compose.override.yml` file and triggers the build.

> [!WARNING]
> You must use *docker's buildkit*, on *Arch Linux* just use `sudo pacman -S docker-buildx` command 

After the build is done, transfer the *tarball* and *install-wow-wotlk.sh* to the steamdeck or server, load the images and run the installation script there.

# Credits

- Installation script: https://github.com/DadsMmoLab/dads-mmo-lab
- AzerothCore: https://github.com/mod-playerbots/azerothcore-wotlk.git
- mod-player-bots: https://github.com/mod-playerbots/mod-playerbots.git
