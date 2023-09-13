# ⚠️ Important note
While the Peacock server officially support Linux, the patcher does not officially support Linux/Proton.
This guide is provided as is, it may not work with everyone's Linux setup and may require extra changes to make it work for you.

Because we're not Linux experts, we cannot guarantee to be able to help you fix your issues. You may check [our Discord](https://thepeacockproject.org/discord) in the Linux megathread help channel.
We're also open to pull requests and tips on how to improve this guide!

# Setup H3 for Peacock with Steam on Linux (and Steam Deck)

Instructions:
- Clone this git repository
- Download and extract Peacock in the `linux-steam-setup/Peacock` folder
    - The `chunk0.js` file must be available at the following path `linux-steam-setup/Peacock/chunk0.js`
- Run the `install-node.sh` script to install Node for Peacock
- Edit HITMAN 3 launch options in Steam to `PROTON_LOG=1 PROTON_DUMP_DEBUG_COMMANDS=1 %command%`
- Start and close the game's launcher from Steam
- Go to the game install location, folder should contain the Launcher.exe
    - Copy the `PeacockPatcher.exe` from inside the Peacock folder and the `WineLaunch.bat` from `linux-steam-setup` and paste them into the game's install location.
- Copy the `run` file from `/tmp/proton_USERNAME/` into `linux-steam-setup` folder
- Open the `run` file with your favorite editor, locate the line starting with `DEF_CMD` and replace `Launcher.exe` to `WineLaunch.bat`
- Click the `Hitman 3 (Peacock)` shortcut to start the game with Peacock
- Edit Peacock Patcher Server address to `127.0.0.1:3000`
- Game is ready to play with Peacock

## Start it from Steam

- Add your Terminal app as a non-Steam game
    - Use Konsole for the Steam Deck
- Open the shortcut properties and change the following:
    - `Start in` to the folder where the `start.sh` script is. (eg: `"/home/deck/linux-steam-setup/"`, quotation marks are necessary)
    - `Launch options` to the following `-e "./start.sh"` (quotation marks are necessary)
- You will now be able to launch Peacock from Steam (or Game Mode on the Steam Deck)
