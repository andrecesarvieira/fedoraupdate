#!/usr/bin/bash
set -euo pipefail

project_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$project_dir/Cargo.toml" | head -n1)
topdir=${RPMBUILD_TOPDIR:-"$project_dir/rpmbuild"}
source_archive="$topdir/SOURCES/fedoraupdate-$version.tar.gz"

mkdir -p "$topdir/BUILD" "$topdir/BUILDROOT" "$topdir/RPMS" "$topdir/SOURCES" "$topdir/SPECS" "$topdir/SRPMS"

tar --directory "$project_dir" \
  --transform "s,^,fedoraupdate-$version/," \
  --exclude target --exclude rpmbuild --exclude .git \
  -czf "$source_archive" \
  Cargo.toml Cargo.lock src data LICENSE README.md

cp "$project_dir/fedoraupdate.spec" "$topdir/SPECS/fedoraupdate.spec"
rpmbuild -ba --define "_topdir $topdir" "$topdir/SPECS/fedoraupdate.spec"

find "$topdir/RPMS" -type f -name '*.rpm' -print
