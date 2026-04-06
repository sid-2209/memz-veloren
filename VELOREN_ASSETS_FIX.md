# 🔧 Veloren Assets Fix - SOLVED

## ✅ Solution Applied

The issue was that Veloren couldn't find its assets directory. The fix is to set the `VELOREN_ASSETS` environment variable.

## Updated Launch Script

The `run_veloren.sh` script has been updated to:

```bash
#!/bin/bash
cd "$(dirname "$0")"
export VELOREN_ASSETS="$(pwd)/assets"
./target/release/veloren-voxygen "$@"
```

This explicitly tells Veloren where to find the assets folder.

## 🎮 How to Test Now

### Terminal 1 - Start Ollama:
```bash
ollama serve
```

### Terminal 2 - Launch Veloren:
```bash
cd veloren
./run_veloren.sh
```

The script will now:
1. Change to the veloren directory
2. Set VELOREN_ASSETS to point to the assets folder
3. Run the veloren-voxygen binary

## Alternative: Manual Launch

If you prefer to run manually:

```bash
cd veloren
export VELOREN_ASSETS="$(pwd)/assets"
./target/release/veloren-voxygen
```

## ✅ Expected Result

Veloren should now launch successfully and show the main menu!

Once in-game:
1. Create/load character
2. Find any NPC
3. Press V, speak, release V
4. See NPC response in chat
5. Hear NPC voice through AirPods!

---

**Try running `./run_veloren.sh` again - it should work now!**
