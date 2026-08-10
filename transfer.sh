#!/usr/bin/env bash

TARGET_IP="${TARGET_IP:?TARGET_IP environment variable is required}"
TARGET_PORT="${TARGET_PORT:-22}"
TARGET_USER="${TARGET_USER:?TARGET_USER environment variable is required}"
TARGET_DIR="~/"

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

INSTALL_SCRIPT="install-wow-wotlk.sh"
SERVER_DIR="wow-server-playerbots"

if ! command -v rsync &> /dev/null; then
    panic "no rsync installed"
fi

rsync -avP -e "ssh -p ${TARGET_PORT}" "$SERVER_DIR" "${TARGET_USER}@${TARGET_IP}:${TARGET_DIR}"
rsync -avP -e "ssh -p ${TARGET_PORT}" "$INSTALL_SCRIPT" "${TARGET_USER}@${TARGET_IP}:${TARGET_DIR}"
