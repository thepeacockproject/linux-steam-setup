# Setup H3 for Peacock with Steam on Linux (and Steam Deck)

Instructions:
- Download and extract Peacock in the `Peacock` folder
    - The `chunk0.js` file must be available at the following path `./Peacock/chunk0.js`
- Run the `install-node.sh` script to install Node in the folder
- Edit HITMAN 3 launch options in Steam to `PROTON_LOG=1 PROTON_DUMP_DEBUG_COMMANDS=1 %command%`
- Start and close the game's launcher from Steam
- Go to the game install location, folder should contain the Launcher.exe
    - Copy the `PeacockPatcher.exe` and the `WineLaunch.bat` files in the folder
- Copy the `run` file from `/tmp/proton_USERNAME/` into peacock folder
- Edit `DEF_CMD` in `run` file, replace `Launcher.exe` to `WineLaunch.bat`
- Click the `Hitman 3 (Peacock)` shortcut to start the game with Peacock

## Start it from Steam

- Add your Terminal app as a non-Steam game
    - Use Konsole for the Steam Deck
- Open the shortcut properties and change the following:
    - `Start in` to the folder where the `start.sh` script is. (eg: `"/home/deck/peacock/"`)
    - `Launch options` to the following `-e "./start.sh"`
