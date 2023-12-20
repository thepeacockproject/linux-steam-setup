#!/bin/bash

# Setup the path to include local node
PATH=node/bin:$PATH

# Get the directory of the script
SCRIPT_DIR=$(dirname "$(readlink -f "$0")")
pushd $SCRIPT_DIR

# Define ANSI escape codes for text formatting
BOLD='\e[1m'
RESET='\e[0m'
RED='\e[31m'
GREEN='\e[32m'
BLUE='\e[34m'

# Success message function
success_message() {
  echo -e "[${GREEN}${BOLD}✔${RESET}] $1!"
}

# Error message function
error_message() {
  echo -e "[${RED}${BOLD}Error${RESET}] $1"
}

# Information message function
info_message() {
  echo -e "\n[${BLUE}${BOLD}Info${RESET}] $1"
}

# Grab Peacock if needed
if [ ! -f "./Peacock/chunk0.js" ]; then
    LATEST_RELEASE=$(curl -L -s -H 'Accept: application/json' https://github.com/thepeacockproject/peacock/releases/latest | sed -e 's/.*"tag_name":"\([^"]*\)".*/\1/')
    FOLDER_NAME="Peacock-${LATEST_RELEASE}-lite"
    FILE_NAME="${FOLDER_NAME}.zip"

    info_message "Grabbing Peacock"

    curl -sSLJO -H "Accept: application/octet-stream" "https://github.com/thepeacockproject/Peacock/releases/download/${LATEST_RELEASE}/${FILE_NAME}" \
    && unzip -q ${FILE_NAME} \
    && rm ${FILE_NAME} \
    && mv ${FOLDER_NAME} Peacock

    if [ $? -eq 0 ]; then
        success_message "Peacock downloaded"
    else
        error_message "We hit a problem getting Peacock"
    fi
else
    success_message "Peacock already installed"
fi

# Copy files
STEAM_DIR=~/.local/share/Steam

STEAM_PATHS=$(cat "${STEAM_DIR}/steamapps/libraryfolders.vdf" | grep -oP '"path"\s+"\K[^"]+')

HITMAN_FOUND=false
for i in $STEAM_PATHS; do
    if [ -d "${i}/steamapps/common/HITMAN 3" ]; then
        HITMAN_FOUND=true
        # Hitman installed here
        info_message "Found Hitman 3 in '${i}/steamapps/common/HITMAN 3', copying needed files"
        cp Peacock/PeacockPatcher.exe "${i}/steamapps/common/HITMAN 3/" \
        && cp WineLaunch.bat "${i}/steamapps/common/HITMAN 3/"

        if [ $? -eq 0 ]; then
            success_message "Copied files"
        else
            error_message "We hit a problem copying files"
        fi
    fi
done

if ! $HITMAN_FOUND; then
    error_message "Couldn't find HITMAN 3. Ensure it's installed correctly, if this is an error then copy the following files yourself\n- Peacock/PeacockPatcher.exe\n- WineLaunch.bat\n\nTo the same directory as HITMAN 3's Launcher.exe\n\n"
fi

# Install node if needed
if ! command -v node &> /dev/null && [ ! -d "./node" ]; then
    info_message "Grabbing node"
    node_version=$(cat Peacock/.nvmrc)
    node_url="https://nodejs.org/dist/$node_version/node-$node_version-linux-x64.tar.gz"

    # Install node
    mkdir node
    curl -sS $node_url | tar --strip-components=1 -C node -zxf  -

    if [ $? -eq 0 ]; then
        success_message "Installed node"
    else
        error_message "We hit a problem getting node"
    fi
else
    success_message "NodeJS already installed"
fi

info_message "Starting Peacock"
PORT=3000 node ./Peacock/chunk0.js

trap popd EXIT
