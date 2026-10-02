#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
version=$(python3 scripts/distribution/version.py)
out="$PWD/store-artifacts"
mkdir -p "$out/aur"
source_name="cranamp-${version}-source.tar.gz"
source_dir=$(mktemp -d)
trap 'rm -rf "$source_dir"' EXIT
git archive --format=tar --prefix="cranamp-${version}/" HEAD | tar -xf - -C "$source_dir"
cp target/distribution/THIRD-PARTY.html "$source_dir/cranamp-${version}/docs/third-party/THIRD-PARTY.html"
tar -czf "$out/$source_name" -C "$source_dir" "cranamp-${version}"
digest=$(sha256sum "$out/$source_name" | cut -d' ' -f1)
cat > "$out/aur/PKGBUILD" <<EOF
pkgname=cranamp
pkgver=$version
pkgrel=1
pkgdesc='Music player in Rust with WSZ skins and an agent-connected Skin Studio'
arch=('x86_64' 'aarch64')
url='https://github.com/samoylenkodmitry/cranamp'
license=('Apache-2.0')
options=('!debug' '!lto')
depends=('alsa-lib' 'gcc-libs' 'glibc')
optdepends=('vulkan-driver: GPU rendering' 'xdg-desktop-portal: file dialogs')
makedepends=('cargo' 'git' 'pkgconf')
source=("https://github.com/samoylenkodmitry/cranamp/releases/download/v\$pkgver/cranamp-\$pkgver-source.tar.gz")
sha256sums=('$digest')
prepare() {
  cd "cranamp-\$pkgver"
  cargo fetch --locked
}
build() {
  cd "cranamp-\$pkgver"
  cargo build --frozen --release --features store
}
package() {
  cd "cranamp-\$pkgver"
  install -Dm755 target/release/cranamp "\$pkgdir/usr/bin/cranamp"
  install -Dm644 platform/linux/io.cranamp.app.desktop "\$pkgdir/usr/share/applications/io.cranamp.app.desktop"
  install -Dm644 assets/icon/icon-512.png "\$pkgdir/usr/share/icons/hicolor/512x512/apps/io.cranamp.app.png"
  install -Dm644 LICENSE "\$pkgdir/usr/share/licenses/cranamp/LICENSE"
  install -Dm644 docs/third-party/THIRD-PARTY.html "\$pkgdir/usr/share/licenses/cranamp/THIRD-PARTY.html"
  install -Dm644 docs/third-party/LiberationSans-OFL-1.1.txt "\$pkgdir/usr/share/licenses/cranamp/LiberationSans-OFL-1.1.txt"
}
EOF
# Generate .SRCINFO with makepkg on Arch; never maintain it separately.
