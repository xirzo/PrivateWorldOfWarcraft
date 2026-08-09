#!/usr/bin/env bash

SERVER_DIR="$(pwd)/wow-server-playerbots"

blue="\033[0;34m"
red="\033[0;31m"
nocolor="\033[0m"

info() {
    echo -e "${blue}$1${nocolor}"
}

panic() {
    echo -e "${red}$1${nocolor}"
    exit 1
}

info "Cloning Playerbots source..."
git clone \
    https://github.com/mod-playerbots/azerothcore-wotlk.git \
    --branch=Playerbot \
    "$SERVER_DIR"

mkdir -p "$SERVER_DIR/modules"

info "Cloning mod-playerbots module..."
git clone --depth 1 \
    https://github.com/mod-playerbots/mod-playerbots.git \
    --branch=master \
    "$SERVER_DIR/modules/mod-playerbots"

info "Creating Docker Compose override configuration..."
cat > "$SERVER_DIR/docker-compose.override.yml" << 'OVERRIDE'
services:
  ac-worldserver:
    build:
      context: .
      target: worldserver
    volumes:
      - ./modules:/azerothcore/modules
    environment:
      AC_PLAYERBOTS_UPDATES_ENABLE_DATABASES: "1"
      AC_AI_PLAYERBOT_RANDOM_BOT_AUTOLOGIN: "1"
      AC_AI_PLAYERBOT_MIN_RANDOM_BOTS: "1600"
      AC_AI_PLAYERBOT_MAX_RANDOM_BOTS: "2000"
  ac-authserver:
    build:
      context: .
      target: authserver
  ac-db-import:
    build:
      context: .
      target: db-import
  ac-client-data-init:
    build:
      context: .
      target: client-data
OVERRIDE

info "Compiling server images (this will take 2-4 hours)..."
cd "$SERVER_DIR" || exit 1
docker compose build

docker save acore/ac-wotlk-worldserver:master acore/ac-wotlk-authserver:master acore/ac-wotlk-db-import:master acore/ac-wotlk-client-data:master > wow-compiled-images.tar
