#!/usr/bin/env python3
"""Validate release icons without requiring a platform packager."""

from pathlib import Path
import struct
import tomllib


ROOT = Path(__file__).resolve().parents[1]
ICON_DIR = ROOT / "packaging" / "icons"


def read(path: Path) -> bytes:
    data = path.read_bytes()
    assert data, f"empty packaging asset: {path.relative_to(ROOT)}"
    return data


def main() -> None:
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text())
    patterns = manifest["package"]["metadata"]["packager"]["icons"]
    for suffix in (".png", ".ico", ".icns"):
        assert any(pattern.endswith(suffix) for pattern in patterns), (
            f"cargo-packager icons do not include {suffix} assets"
        )

    png = read(ICON_DIR / "1024x1024.png")
    assert png[:8] == b"\x89PNG\r\n\x1a\n", "invalid PNG app icon"
    width, height = struct.unpack(">II", png[16:24])
    assert (width, height) == (1024, 1024), "app PNG must be 1024x1024"

    ico = read(ICON_DIR / "plicowarden.ico")
    assert ico[:4] == b"\x00\x00\x01\x00", "invalid Windows ICO app icon"

    icns = read(ICON_DIR / "plicowarden.icns")
    assert icns[:4] == b"icns", "invalid macOS ICNS app icon"
    assert struct.unpack(">I", icns[4:8])[0] == len(icns), "invalid ICNS length"

    print("packaging icons: PNG, ICO and ICNS are valid")


if __name__ == "__main__":
    main()
