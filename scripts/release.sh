#!/bin/sh
# Prepare a release: set the version in Cargo.toml and regenerate the changelog.
# Review the diff, then commit and tag.
set -eu

cd "$(dirname "$0")/.."

version=${1:-}
case "$version" in
    v[0-9]*) ;;
    *)
        echo "usage: scripts/release.sh vX.Y.Z" >&2
        exit 2
        ;;
esac
number=${version#v}

# shellcheck source=scripts/versions.env
. scripts/versions.env

# Resolve the pinned build rather than whatever PATH finds first, so the notes
# do not depend on which machine generated them.
cliff="${CARGO_HOME:-$HOME/.cargo}/bin/git-cliff"
[ -x "$cliff" ] || cliff=$(command -v git-cliff || true)
[ -n "$cliff" ] || {
    echo "git-cliff is required: scripts/install-dev-tools.sh" >&2
    exit 1
}
have=$("$cliff" --version | awk '{print $2}')
[ "$have" = "$GIT_CLIFF_VERSION" ] || {
    echo "git-cliff $have found, $GIT_CLIFF_VERSION pinned: scripts/install-dev-tools.sh" >&2
    exit 1
}
[ -z "$(git status --porcelain)" ] || {
    echo "working tree is dirty" >&2
    exit 1
}
git rev-parse "$version" >/dev/null 2>&1 && {
    echo "$version already exists; bump instead of moving it" >&2
    exit 1
}

sed -i "0,/^version = \".*\"/s//version = \"$number\"/" Cargo.toml
cargo update --workspace --quiet

"$cliff" --config cliff.toml --tag "$version" -o CHANGELOG.md

echo
echo "Prepared $version. Review, then:"
echo "  git commit -am 'chore(release): $version'"
echo "  git tag -a $version -m 'dat $number'"
echo "  git push && git push origin $version"
