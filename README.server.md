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

## Networking

In order for players to connect from outside your *LAN*, you need to forward the ports on your router.

[AzerothCore](https://github.com/mod-playerbots/azerothcore-wotlk.git) suggests forwarding two mandatory *TCP* ports:

* **3724** (for the authserver)
* **8085** (for the worldserver)

> [!WARNING]
> Do not open your MySQL port publicly to the internet. If you need to connect to your database from an external machine, use a SSH tunnel

More info: [https://www.azerothcore.org/wiki/networking](https://www.azerothcore.org/wiki/networking)

### Configuring Database Realmlist IP

You need to make sure that your authserver application directs incoming connections to your realm by setting the correct IP address in the database.

#### 1. Retrieve MySQL Root Password

Find your running database container and inspect its environment variables to retrieve the password:

```sh
docker exec $(docker ps --format '{{.Names}}' | grep -i "ac-database" | head -1) env | grep MYSQL_ROOT_PASSWORD
```

*(Default root password is usually `password`)*

#### 2. Connect to the MySQL Shell

Connect directly to the database container's MySQL CLI:

```sh
docker exec -it $(docker ps --format '{{.Names}}' | grep -i "ac-database" | head -1) mysql -u root -p
```

#### 3. Update Realmlist Table

Once inside the MySQL shell, select the `acore_auth` database and update your realm's address:

```sql
USE acore_auth;

SELECT id, name, address FROM realmlist;

UPDATE realmlist SET address = 'YOUR_IP_HERE' WHERE id = 1;
```

Replace `'YOUR_IP_HERE'` based on your network environment:

- **`127.0.0.1`**: If AzerothCore and your WoW client are on the same machine.
- **LAN IP (`192.168.x.x`)**: If hosting on a separate PC on your local home network.
- **Public IP**: If allowing players outside your home network to connect.

## Changing server locale

Edit `~/wow-server-playerbots/env/dist/etc/worldserver.conf`, find *RealmZone
= xxx* and set to wanted locale, optionally change *DBC.Locale* too.

## Steam Customization

You may find nice banners/backgrounds/icons here: https://www.steamgriddb.com/search/grids?term=World+of+warcraft

# Credits

- Installation script: https://github.com/DadsMmoLab/dads-mmo-lab
- AzerothCore: https://github.com/mod-playerbots/azerothcore-wotlk.git
- mod-player-bots: https://github.com/mod-playerbots/mod-playerbots.git
- steamgriddb: https://www.steamgriddb.com/search/grids?term=World+of+warcraft
