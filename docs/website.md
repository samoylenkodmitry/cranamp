# Website deployment

The landing page, privacy policy and support page are static files in `site/`.
The VPS serves that directory directly from this repository. No Rust, Node, or
website build is needed. The web player remains on GitHub Pages and is built by
the Pages workflow.

## First deployment

The existing VPS reverse proxy uses `nginxproxy/nginx-proxy` and
`nginxproxy/acme-companion` on the Docker network `net`. DNS for
`cranamp.dmitrysamoylenko.in` must point to that VPS.

```sh
ssh vps
git clone https://github.com/samoylenkodmitry/cranamp.git ~/cranamp
cd ~/cranamp
docker compose up -d
```

The proxy discovers the hostname from the Compose environment and obtains the
HTTPS certificate. Only `site/` is mounted into the web server; repository source,
Git metadata, build files and credentials are not served.

## Update

```sh
ssh vps
cd ~/cranamp
git pull --ff-only
docker compose up -d
docker compose exec cranamp-landing nginx -t
```

Static file changes are visible immediately through the read-only bind mount.
After changing `deploy/nginx.conf`, recreate the service so the single-file
bind mount uses the new file, then validate the configuration:

```sh
docker compose up -d --force-recreate cranamp-landing
docker compose exec cranamp-landing nginx -t
```

Keep the untracked `site/review/` directory during deployment. It holds review
media and is separate from the landing page.

Verify `/`, `/privacy/`, `/support/`, images and HTTPS after deploying. The
download section links the App Store, Google Play, Microsoft Store and AUR
listings, which were public on 13 October 2026.

The landing page has no JavaScript, third-party fonts, analytics or cookies.
The site container disables access logging; the VPS reverse proxy and host may
still retain operational logs as described by the privacy policy.

## Images

The page uses unaltered PNG files copied from the
[0.1.92 release gallery](../store/releases/0.1.92/screenshots.json). That manifest
records source versions, capture methods and SHA-256 hashes.

| Website asset in `site/assets/` | Source under `store/releases/0.1.92/screenshots/` |
| --- | --- |
| `player-0.1.92.png` | `macos/00-playback.png` |
| `studio-desktop-0.1.92.png` | `macos/01-studio-canvas.png` |
| `studio-phone-0.1.92.png` | `android-phone/02-studio-canvas.png` |
| `studio-tablet-0.1.92.png` | `android-tablet-10/02-studio-canvas.png` |
| `studio-colours-0.1.92.png` | `macos/03-skin-colours.png` |
| `studio-mcp-0.1.92.png` | `macos/05-agent-connection.png` |

The hero's SVG viewport hides the black side margins in the player capture.
The linked full-size file keeps all source pixels. Studio images show the
complete capture and open at full size when selected. The desktop canvas is
also the Open Graph image. `icon.png` comes from `assets/icon/icon-512.png`.

Preview the repository with `python3 -m http.server 8766 --bind 127.0.0.1` and open
`http://127.0.0.1:8766/site/`. Check the page at phone, tablet and desktop widths.
