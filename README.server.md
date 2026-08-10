# How To

*build_docker_image.sh* is used to offload building the server to a better PC, as I don't wanna do what on a steamdeck.

It just clones [AzerothCore](https://github.com/mod-playerbots/azerothcore-wotlk.git) and [mod-player-bots](https://github.com/mod-playerbots/mod-playerbots.git), then generates a `docker-compose.override.yml` file and triggers the build.

## Build

Run *build_docker_image.sh* and wait for some time for it to build the images.

> [!WARNING]
> You must use *docker's buildkit*. On *Arch Linux* use `sudo pacman -S docker-buildx` command:
> 
> ```sh
> ./build_docker_image.sh 
> ```

## Transfer

After the build is done, use `transfer.sh` to copy the *server directory* and *install-wow-wotlk.sh* to the target machine. You must provide the target IP as an environment variable. 

```sh
export TARGET_IP="192.168.1.100"
export TARGET_PORT="22" # Optional
./transfer.sh
```

Once transferred, load the images and initialize the containers

## Installing

Load the saved Docker images:

```sh
docker load < ~/wow-server-playerbots/wow-compiled-images.tar
```

Change to the server directory and start the containers without rebuilding:

```sh
cd ~/wow-server-playerbots
docker compose up --no-build --no-start
```

and run the installation script

```sh
cd ~/
./install-wow-wotlk.sh
```

Answer *y* to the script prompts until step 2. When it asks about building images, accept – it will detect that the images are already built.

Proceed with the guide for server (you don't need to install neither *GE-Proton* nor client). [Guide](https://github.com/DadsMmoLab/dads-mmo-lab)

## Forwarding ports

In order for players to connect not from your *LAN*, you need to forward the
ports on your router.

[AzerothCore](https://github.com/mod-playerbots/azerothcore-wotlk.git) suggests to forward two mandatory *TCP* ports: 

- 3724 (for the authserver) 
- 8085 (for the worldserver)

More info: https://www.azerothcore.org/wiki/networking

## Steam Customization

You may find nice banners/backgrounds/icons here: https://www.steamgriddb.com/search/grids?term=World+of+warcraft

# Credits

- Installation script: https://github.com/DadsMmoLab/dads-mmo-lab
- AzerothCore: https://github.com/mod-playerbots/azerothcore-wotlk.git
- mod-player-bots: https://github.com/mod-playerbots/mod-playerbots.git
- steamgriddb: https://www.steamgriddb.com/search/grids?term=World+of+warcraft
