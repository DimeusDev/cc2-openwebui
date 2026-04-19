# cc2-openwebui

Simple LAN web UI for Elegoo CC2.

Linux: one container runs both the Rust app (frontend included) and Obico ML.

## Current Issues
This project just started and it already took quite a lot of my time so its not 100% finished..
- No Windows support (comming soon! I just need to get a windows pc to try)
- Upload not working
- Axis Control need to be made
- Canvas filament management need to be made
- Start print not fully working
- UI is not really responsive (will be fixed dw)
- UI looks ai cause i kinda suck at styling but ill try to remake it all myself ^^

## Features
- Automatic printer recognition
- Full Web UI
- Customisable AI print failures detection with Obico ML
- Notifications support (NTFY and Discord webhook)

## Run (Docker Compose)

```bash
docker compose up -d --build
```

Then open `http://127.0.0.1:8484` and do setup from onboarding.
No manual config copy is needed.

## Tips
- Use tailscale/others to use the webui from everywhere outside your network.

## Notes

- Data, snapshots, and config are kept in `/work` (mounted volume above)
- Obico ML runs inside the same container on port `3333`

### Contact:
_dimeus on discord
pro@dimeus.dev
