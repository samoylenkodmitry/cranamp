# Website deployment

The Skin Studio guide, privacy policy and support page are static files in `site/`.
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

The guide has no JavaScript, third-party fonts, analytics or cookies.
The site container disables access logging; the VPS reverse proxy and host may
still retain operational logs as described by the privacy policy.

## Guide structure

The home page is an editing guide with eleven sections. Desktop uses a contents
sidebar; small screens use a collapsible contents list. Drawing tools and optional
tasks use native HTML details controls. Every screenshot links to its full-size
file. The page uses no JavaScript.

Keep the steps tied to visible control names. Check navigation against the current
UI before publishing: standalone and player-hosted Studio have different save
options, and desktop Preview uses state thumbnails. File actions start from the
save icon beside the canvas or the share icon in Preview.

## Screenshots

[`site/assets/guide/captures.json`](../site/assets/guide/captures.json) records
37 new captures: source commit, capture case, dimensions and SHA-256 for each file.
They cover every drawing tool, paint layers, all four active/pressed combinations,
Skin settings, preview and save screens. Phone examples show the canvas, brush
colour and visualizer controls; a tablet example shows the wider canvas.

The existing `cranamp-studio-review` test executable was compiled from the recorded
commit with `--locked --features studio-review`. Its named cases use the real
editor and a temporary bundled-skin document. The capture API saves actual GPU
pixels. Only the separate review footer is excluded; converting fully opaque RGBA
to RGB changes no visible pixels. No user document or local pathname is shown.

The guide also uses two existing 0.1.92 release captures:

- `site/assets/player-0.1.92.png`: `macos/00-playback.png` in the release gallery.
  An SVG viewport shows the top player window. An HTML/SVG circle marks the
  Settings click area; the screenshot itself is unchanged.
- `site/assets/studio-mcp-0.1.92.png`: `macos/05-agent-connection.png` in the release
  gallery. It shows the desktop server after connection.

The [release manifest](../store/releases/0.1.92/screenshots.json) records their
original provenance. Older versioned image URLs remain available for existing
links. The app icon is `assets/icon/icon-512.png`.

Preview with `python3 -m http.server 8766 --bind 127.0.0.1` and open
`http://127.0.0.1:8766/site/`. Check contents links and tool details with keyboard
and pointer input. Inspect phone, tablet and desktop widths. Validate local links,
image hashes and screenshot privacy before deployment.
