#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
version=$(python3 scripts/distribution/version.py)
work="target/distribution/linux"
stage="$work/root"
out="$PWD/store-artifacts"
rm -rf "$work"
mkdir -p "$stage/usr/bin" "$stage/usr/share/applications" \
  "$stage/usr/share/icons/hicolor/512x512/apps" "$stage/usr/share/licenses/cranamp" "$out"
cargo build --locked --release --features store
install -m755 target/release/cranamp "$stage/usr/bin/cranamp"
install -m644 platform/linux/io.cranamp.app.desktop "$stage/usr/share/applications/"
install -m644 assets/icon/icon-512.png "$stage/usr/share/icons/hicolor/512x512/apps/io.cranamp.app.png"
install -m644 LICENSE docs/third-party/LiberationSans-OFL-1.1.txt "$stage/usr/share/licenses/cranamp/"
install -m644 target/distribution/THIRD-PARTY.html "$stage/usr/share/licenses/cranamp/"
# winit and wgpu load these desktop libraries dynamically, so ELF dependency
# scanning alone does not find them.
desktop_deps='libx11-6, libxi6, libxkbcommon-x11-0, libwayland-client0, libvulkan1, hicolor-icon-theme'
# dpkg-shlibdeps derives the linked dependencies from the actual executable.
mkdir -p "$work/debian" "$stage/DEBIAN"
printf 'Source: cranamp\nSection: sound\nPriority: optional\nMaintainer: Cranamp contributors <noreply@github.com>\n' > "$work/debian/control"
(cd "$work" && dpkg-shlibdeps -O root/usr/bin/cranamp) > "$work/dependencies"
deps=$(sed -n 's/^shlibs:Depends=//p' "$work/dependencies")
cat > "$stage/DEBIAN/control" <<EOF
Package: cranamp
Version: $version
Architecture: amd64
Maintainer: Cranamp contributors <noreply@github.com>
Depends: $deps, $desktop_deps
Recommends: mesa-vulkan-drivers, xdg-desktop-portal
Section: sound
Priority: optional
Homepage: https://github.com/samoylenkodmitry/cranamp
Description: Music player in Rust with WSZ skins and a built-in Skin Studio
EOF
dpkg-deb --root-owner-group --build "$stage" "$out/cranamp_${version}_amd64.deb"
# The same payload can be inspected or installed without a package manager.
tar -czf "$out/cranamp-${version}-linux-x86_64-store.tar.gz" -C "$stage" usr
mkdir -p "$work/rpm/BUILD" "$work/rpm/RPMS" "$work/rpm/SOURCES" "$work/rpm/SPECS" "$work/rpm/SRPMS"
cat > "$work/rpm/SPECS/cranamp.spec" <<EOF
Name: cranamp
Version: $version
Release: 1
Summary: Music player with WSZ skins and a built-in Skin Studio
License: Apache-2.0 AND MIT AND MPL-2.0 AND OFL-1.1
URL: https://github.com/samoylenkodmitry/cranamp
BuildArch: x86_64
Requires: libX11.so.6()(64bit)
Requires: libXi.so.6()(64bit)
Requires: libxkbcommon-x11.so.0()(64bit)
Requires: libwayland-client.so.0()(64bit)
Requires: libvulkan.so.1()(64bit)
Requires: hicolor-icon-theme
Recommends: mesa-vulkan-drivers, xdg-desktop-portal
%description
Music player in Rust with custom skins and an agent-connected Skin Studio.
%install
mkdir -p %{buildroot}
cp -a $PWD/$stage/usr %{buildroot}/
%files
/usr/bin/cranamp
/usr/share/applications/io.cranamp.app.desktop
/usr/share/icons/hicolor/512x512/apps/io.cranamp.app.png
/usr/share/licenses/cranamp
EOF
rpmbuild --define "_topdir $PWD/$work/rpm" -bb "$work/rpm/SPECS/cranamp.spec"
cp "$work"/rpm/RPMS/x86_64/*.rpm "$out/"
