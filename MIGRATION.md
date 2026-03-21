# Migration Guide

This guide is for players moving from the old shell-script-based `linux-steam-setup` to the AppImage launcher.

## What Changes

- Peacock and Node.js move into the launcher-managed install directory.
- The old `peacock.service` is replaced with the launcher's service definition.
- If your old service started automatically or was already running, the launcher restores that state.
- Legacy patcher files such as `PeacockPatcher.exe` and `WineLaunch.bat` are removed from detected game folders.
- The old batch-file injection flow is retired. Use ZHMModSDK with OnlineTools after migrating.

## Before You Start

1. Make sure the legacy install still exists on disk.
2. Close HITMAN 3 before starting migration.
3. If you edited your old setup manually, keep the legacy directory until you confirm the new launcher setup works.

## Migration Steps

1. Launch the AppImage.
2. Select **Migrate from old setup**.
3. Confirm the detected folder, or choose **Browse for folder…** if your install lives somewhere else.
4. Start migration.
5. When the wizard finishes, keep the old directory until you have tested the new setup once.

## Switch From The Old Patcher Method

After migration, do not go back to `WineLaunch.bat`, `Hitman 3 (Peacock).desktop`, or `PeacockPatcher.exe`.

Instead:

1. In the launcher, open **Manage ZHMModSDK**.
2. Install ZHMModSDK into your HITMAN 3 directory.
3. Start the game normally through Steam or Heroic.
4. Open the SDK panel in-game and enable **OnlineTools**.
5. In OnlineTools, use **Help** → **Load Old Patcher Settings**.
6. Confirm the server address is `localhost:3000` unless you changed the Peacock port.

If you changed the Peacock port in the launcher settings, use `localhost:PORT` with your selected port instead.

## After Migration

1. Check **Manage Service** in the launcher and confirm the service is installed.
2. If you want Peacock running in the background automatically, ensure **Enable on Boot** is set.
3. Start HITMAN 3 and verify OnlineTools connects successfully.
4. Delete the old legacy directory only after you are satisfied the new setup works.

## Troubleshooting

- If Peacock is not reachable, open **Manage Service** in the launcher and verify the service is running.
- If OnlineTools is missing, reinstall ZHMModSDK from the launcher and restart the game.
- If you still have old patcher files in a custom game folder the launcher did not detect, remove `PeacockPatcher.exe` and `WineLaunch.bat` manually and continue using the normal game launch path.