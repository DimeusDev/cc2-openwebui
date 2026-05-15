# cc2-openwebui

Simple LAN web UI for Elegoo CC2.

Linux: one container runs both the Rust app (frontend included) and Obico ML.

## Current Issues
This project just started and it already took quite a lot of my time so its not 100% finished..
- No Windows support (comming soon! I just need to get a windows pc to try)
- Canvas filament edit (soon)
- UI is not really responsive (for mobiles)

## Features
- Automatic printer recognition
- Full Web UI
- Customisable AI print failures detection with Obico ML
- Notifications support (NTFY and Discord webhook)
- All features from ElegooSlicer UI

## Run (Docker Compose)
> runs both the webui and Obico ML
### Windows (Docker Desktop
```bash
docker compose -f docker-compose.windows.yml up --build
```

### Others
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
