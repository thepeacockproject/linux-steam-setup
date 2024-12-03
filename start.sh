#!/bin/bash

#################################################################

STEAM_DIR=~/.local/share/Steam
SCRIPT_DIR="$(dirname "$(readlink -f "$0")")"
PEACOCK_REPO="https://github.com/thepeacockproject/peacock"
NODE_DIR="${SCRIPT_DIR}/node"
PEACOCK_DIR="${SCRIPT_DIR}/Peacock"
SERVER_PORT=3000

#################################################################

# Define ANSI escape codes for text formatting
BOLD='\e[1m'
RESET='\e[0m'
RED='\e[31m'
GREEN='\e[32m'
BLUE='\e[34m'

# Functions for messages
success_message() { echo -e "[${GREEN}${BOLD}✔${RESET}] $1!"; }
error_message() { echo -e "[${RED}${BOLD}Error${RESET}] $1"; }
info_message() { echo -e "\n[${BLUE}${BOLD}Info${RESET}] $1"; }

# Download Peacock
download_peacock() {
    # If Peacock isn't present, download it
    if [ ! -f "${PEACOCK_DIR}/chunk0.js" ]; then
        local latest_release
        local folder_name
        local file_name

        latest_release=$(curl -L -s -H 'Accept: application/json' "${PEACOCK_REPO}/releases/latest" | sed -e 's/.*"tag_name":"\([^"]*\)".*/\1/')
        folder_name="Peacock-${latest_release}-linux"
        file_name="${folder_name}.zip"

        info_message "Downloading Peacock..."

        curl -sSLJO -H "Accept: application/octet-stream" "${PEACOCK_REPO}/releases/download/${latest_release}/${file_name}" \
            && unzip -q "${file_name}" \
            && rm "${file_name}" \
            && mv "${folder_name}" "${PEACOCK_DIR}"
        
        if [ $? -eq 0 ]; then
            success_message "Peacock downloaded"
        else
            error_message "We hit a problem getting Peacock"
        fi
    else
        success_message "Peacock already installed"
    fi
}

# Find and process Hitman installations
process_hitman_installation() {
    local hitman_found=false
    local paths

    paths=$(grep -oP '"path"\s+"\K[^"]+' "${STEAM_DIR}/steamapps/libraryfolders.vdf")
    for path in $paths; do
        ######
        # Remove the games you don't want Peacock to be installed for from this list ("HITMAN" "HITMAN2" "HITMAN 3")
        ######
        for game in "HITMAN" "HITMAN2" "HITMAN 3"; do
            local game_path="${path}/steamapps/common/${game}"
            if [ -d "${game_path}" ]; then
                hitman_found=true
                info_message "Found ${game} in '${game_path}', copying needed files..."
                cp "${PEACOCK_DIR}/PeacockPatcher.exe" "${game_path}/" \
                    && cp "${SCRIPT_DIR}/WineLaunch.bat" "${game_path}/"
                if [ $? -eq 0 ]; then
                    success_message "Files copied for ${game}"
                else
                    error_message "There was a problem copying files"
                fi
            fi
        done
    done

    # If no Hitman game was found
    if ! $hitman_found; then
        error_message "Couldn't find any HITMAN game. Ensure it's installed correctly, then copy the following files manually:\n- Peacock/PeacockPatcher.exe\n- WineLaunch.bat\n\n to the same folder as HITMAN (2/3)'s Launcher.exe\n\n"
    fi
}

# Install Node.js if needed
install_node() {
    if ! command -v node &> /dev/null && [ ! -d "${NODE_DIR}" ]; then
        info_message "Installing Node.js..."
        local node_version
        node_version=$(cat "${PEACOCK_DIR}/.nvmrc")
        mkdir -p "${NODE_DIR}"
        curl -sS "https://nodejs.org/dist/${node_version}/node-${node_version}-linux-x64.tar.gz" \
            | tar --strip-components=1 -C "${NODE_DIR}" -zxf -
        if [ $? -eq 0 ]; then
            success_message "Installed Node.js"
        else
            error_message "There was a problem getting Node.js"
        fi
    else
        success_message "Node.js already installed"
    fi
}

info_message "Starting Setup..."
pushd "${SCRIPT_DIR}"

download_peacock
process_hitman_installation
install_node

info_message "Starting Peacock server..."
cd "${PEACOCK_DIR}"
PORT=${SERVER_PORT} node chunk0.js
trap popd EXIT
