#!/bin/sh
set -eu

REPO="aduverger/lumiere"
BIN_NAME="lumiere"
TMPDIR_CLEANUP=""
STAGED_CLEANUP=""

cleanup() {
    if [ -n "$STAGED_CLEANUP" ]; then
        rm -f "$STAGED_CLEANUP"
    fi
    if [ -n "$TMPDIR_CLEANUP" ]; then
        rm -rf "$TMPDIR_CLEANUP"
    fi
}

trap cleanup EXIT INT TERM

die() {
    printf 'error: %s\n' "$1" >&2
    exit 1
}

need_cmd() {
    command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

detect_target() {
    os=$(uname -s)
    arch=$(uname -m)

    case "$os" in
        Linux)
            if command -v ldd >/dev/null 2>&1 && ldd --version 2>&1 | grep -qi musl; then
                die "pre-built Linux binaries require glibc (musl is not supported yet); install from source with: cargo install lumiere-cli --locked"
            fi
            os_triple="unknown-linux-gnu"
            ;;
        Darwin)
            if [ "$arch" = "x86_64" ]; then
                if [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || printf '0')" = "1" ]; then
                    arch="aarch64"
                fi
            fi
            os_triple="apple-darwin"
            ;;
        *)
            die "unsupported operating system: $os (Lumiere provides pre-built binaries for Linux and macOS)"
            ;;
    esac

    case "$arch" in
        x86_64|amd64)
            arch_triple="x86_64"
            ;;
        aarch64|arm64)
            arch_triple="aarch64"
            ;;
        *)
            die "unsupported architecture: $arch (expected x86_64 or aarch64/arm64)"
            ;;
    esac

    printf '%s-%s\n' "$arch_triple" "$os_triple"
}

resolve_version() {
    if [ -n "${LUMIERE_VERSION:-}" ]; then
        raw="${LUMIERE_VERSION#v}"
        case "$raw" in
            ""|*[!0-9A-Za-z._+-]*|.*|*..*)
                die "invalid LUMIERE_VERSION: ${LUMIERE_VERSION}"
                ;;
        esac
        printf 'v%s\n' "$raw"
        return
    fi

    latest_url=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/${REPO}/releases/latest") \
        || die "failed to resolve latest release from GitHub"
    tag="${latest_url##*/}"

    case "$tag" in
        v*)
            printf '%s\n' "$tag"
            ;;
        *)
            die "could not determine latest release tag (resolved URL: ${latest_url})"
            ;;
    esac
}

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    else
        die "either sha256sum or shasum is required to verify the download"
    fi
}

verify_checksum() {
    archive_path="$1"
    checksum_path="$2"

    expected=$(awk '{print $1}' "$checksum_path")
    case "$expected" in
        *[!0-9a-fA-F]*|"")
            die "invalid checksum file contents"
            ;;
    esac
    if [ "${#expected}" -ne 64 ]; then
        die "invalid SHA-256 checksum length"
    fi

    actual=$(sha256_file "$archive_path")
    if [ "$expected" != "$actual" ]; then
        die "SHA-256 checksum mismatch (expected ${expected}, got ${actual})"
    fi
}

main() {
    need_cmd curl
    need_cmd tar
    need_cmd awk
    need_cmd mktemp

    target=$(detect_target)
    version=$(resolve_version)
    if [ -n "${LUMIERE_INSTALL_DIR:-}" ]; then
        install_dir="$LUMIERE_INSTALL_DIR"
    else
        [ -n "${HOME:-}" ] || die "HOME is not set; set LUMIERE_INSTALL_DIR explicitly"
        install_dir="${HOME}/.local/bin"
    fi
    if [ "$install_dir" != "/" ]; then
        install_dir="${install_dir%/}"
    fi

    archive="${BIN_NAME}-${version}-${target}.tar.gz"
    checksum="${archive}.sha256"
    base_url="https://github.com/${REPO}/releases/download/${version}"

    TMPDIR_CLEANUP=$(mktemp -d)

    printf 'Downloading %s %s (%s)...\n' "$BIN_NAME" "$version" "$target"
    curl -fsSL -o "${TMPDIR_CLEANUP}/${archive}" "${base_url}/${archive}" \
        || die "failed to download ${base_url}/${archive}"
    curl -fsSL -o "${TMPDIR_CLEANUP}/${checksum}" "${base_url}/${checksum}" \
        || die "failed to download ${base_url}/${checksum}"

    verify_checksum "${TMPDIR_CLEANUP}/${archive}" "${TMPDIR_CLEANUP}/${checksum}"

    tar -xzf "${TMPDIR_CLEANUP}/${archive}" -C "$TMPDIR_CLEANUP"
    [ -f "${TMPDIR_CLEANUP}/${BIN_NAME}" ] || die "archive did not contain ${BIN_NAME} binary"

    mkdir -p "$install_dir"
    staged="${install_dir}/.${BIN_NAME}.tmp.$$"
    STAGED_CLEANUP="$staged"
    cp "${TMPDIR_CLEANUP}/${BIN_NAME}" "$staged"
    chmod 755 "$staged"
    mv -f "$staged" "${install_dir}/${BIN_NAME}"
    STAGED_CLEANUP=""

    printf 'Installed %s %s to %s/%s\n' "$BIN_NAME" "$version" "$install_dir" "$BIN_NAME"

    case ":${PATH:-}:" in
        *":${install_dir}:"*)
            ;;
        *)
            printf '\nNote: %s is not in your PATH. Add it to your shell profile:\n' "$install_dir"
            printf '  export PATH="%s:$PATH"\n' "$install_dir"
            ;;
    esac
}

main "$@"
