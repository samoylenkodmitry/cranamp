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
After changing `deploy/nginx.conf`, run
`docker compose exec cranamp-landing nginx -s reload`.

Verify `/`, `/privacy/`, `/support/`, images and HTTPS after deploying. Only add
store download links once those listings are publicly available.

The landing page has no JavaScript, third-party fonts, analytics or cookies.
The site container disables access logging; the VPS reverse proxy and host may
still retain operational logs as described by the privacy policy.

## Images

- `site/assets/player.jpg`: actual v0.1.86 web player capture with the original
  Catamp Silverplay skin and the project's generated demo music.
- `site/assets/studio.png`: the existing real Studio GPU capture from
  `docs/images/studio-layers.png`; no artwork was redrawn for the page.
- `site/assets/icon.png`: original app icon from `assets/icon/icon-512.png`.
- `site/assets/social.jpg`: the store feature graphic from `store/assets/`.

Preview the repository with `python3 -m http.server 8766 --bind 127.0.0.1` and open
`http://127.0.0.1:8766/site/`.
