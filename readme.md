# cc2-openwebui

Simple LAN web UI for Elegoo CC2.

Linux: one container runs both the Rust app (frontend included) and Obico ML.

## Current Issues
This project just started and it already took quite a lot of my time so its not 100% finished..
- No Windows support (comming soon!)
- Upload not working
- Axis Control need to be made
- Canvas filament management need to be made
- Start print not fully working

## Run with Docker

```bash
docker build -t cc2-openwebui .
docker run -d \
  --name cc2-openwebui \
  --restart unless-stopped \
  --network host \
  -v cc2_openwebui_state:/work \
  cc2-openwebui
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