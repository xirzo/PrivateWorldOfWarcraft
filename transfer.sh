#!/usr/bin/env bash

TARGET_IP="${TARGET_IP:?TARGET_IP environment variable is required}"
TARGET_PORT="${TARGET_PORT:-22}"
TARGET_USER="${TARGET_USER:-xir}"
TARGET_DIR="~/"

TARBALL="wow-compiled-images.tar"
INSTALL_SCRIPT="install-wow-wotlk.sh"
SERVER_DIR="wow-server-playerbots"

if ! command -v rsync &> /dev/null; then
    exit 1
fi

if [ ! -f "$TARBALL" ]; then
    exit 1
fi

rsync -avP -e "ssh -p ${TARGET_PORT}" "$SERVER_DIR" "${TARGET_USER}@${TARGET_IP}:${TARGET_DIR}"
rsync -avP -e "ssh -p ${TARGET_PORT}" "$INSTALL_SCRIPT" "${TARGET_USER}@${TARGET_IP}:${TARGET_DIR}"
rsync -avP -e "ssh -p ${TARGET_PORT}" "$TARBALL" "${TARGET_USER}@${TARGET_IP}:${TARGET_DIR}"
