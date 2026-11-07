#!/usr/bin/env python3
"""Pack tracelamp and recon into one archive for a release target.

The archive name is tracelamp-<version>-<target>.tar.gz, or .zip on Windows.
Both command names are included. LICENSE, NOTICE, and README.md ride along.
"""

from __future__ import annotations

import argparse
import hashlib
import stat
import tarfile
import tempfile
import zipfile
from pathlib import Path

BINS = ("tracelamp", "recon")
DOCS = ("LICENSE", "NOTICE", "README.md")


def cargo_version(root: Path) -> str:
    for line in (root / "Cargo.toml").read_text(encoding="utf-8").splitlines():
        if line.startswith("version "):
            return line.split("=", 1)[1].strip().strip('"')
    raise SystemExit(f"no version in {root / 'Cargo.toml'}")


def is_windows(target: str) -> bool:
    return target.endswith("windows-msvc") or target.endswith("windows-gnu")


def package(root: Path, target: str, bin_dir: Path, out_dir: Path, version: str | None) -> Path:
    crate = cargo_version(root)
    version = version or crate
    if version != crate:
        raise SystemExit(f"version {version} does not match Cargo.toml {crate}")
    windows = is_windows(target)
    suffix = ".exe" if windows else ""
    out_dir.mkdir(parents=True, exist_ok=True)
    archive_name = f"tracelamp-{version}-{target}" + (".zip" if windows else ".tar.gz")
    archive_path = out_dir / archive_name

    missing = [bin_dir / f"{name}{suffix}" for name in BINS if not (bin_dir / f"{name}{suffix}").is_file()]
    if missing:
        raise SystemExit("missing binaries: " + ", ".join(str(path) for path in missing))
    for name in DOCS:
        if not (root / name).is_file():
            raise SystemExit(f"missing {root / name}")

    if windows:
        with zipfile.ZipFile(archive_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for name in BINS:
                filename = f"{name}{suffix}"
                archive.write(bin_dir / filename, arcname=filename)
            for name in DOCS:
                archive.write(root / name, arcname=name)
    else:
        with tarfile.open(archive_path, "w:gz") as archive:
            for name in BINS:
                filename = f"{name}{suffix}"
                source = bin_dir / filename
                info = archive.gettarinfo(source, arcname=filename)
                info.mode = stat.S_IRWXU | stat.S_IRGRP | stat.S_IXGRP | stat.S_IROTH | stat.S_IXOTH
                with source.open("rb") as handle:
                    archive.addfile(info, handle)
            for name in DOCS:
                archive.add(root / name, arcname=name)

    digest = hashlib.sha256(archive_path.read_bytes()).hexdigest()
    print(f"{digest}  {archive_path.name}")
    return archive_path


def self_test() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "Cargo.toml").write_text('[package]\nname = "tracelamp"\nversion = "9.9.9"\n', encoding="utf-8")
        for name in DOCS:
            (root / name).write_text(name + "\n", encoding="utf-8")
        bin_dir = root / "bins"
        bin_dir.mkdir()
        for name in BINS:
            (bin_dir / name).write_bytes(b"#!/bin/sh\necho ok\n")
        out = root / "out"
        archive = package(root, "x86_64-unknown-linux-gnu", bin_dir, out, "9.9.9")
        with tarfile.open(archive, "r:gz") as packed:
            names = set(packed.getnames())
        expected = {"tracelamp", "recon", *DOCS}
        if names != expected:
            raise SystemExit(f"tar contents {names} != {expected}")
        mode = packed_mode(archive, "tracelamp")
        if mode & 0o111 == 0:
            raise SystemExit(f"tracelamp is not executable in the archive ({oct(mode)})")

        win_dir = root / "win"
        win_dir.mkdir()
        for name in BINS:
            (win_dir / f"{name}.exe").write_bytes(b"MZ")
        win_archive = package(root, "x86_64-pc-windows-msvc", win_dir, out, None)
        with zipfile.ZipFile(win_archive) as packed:
            win_names = set(packed.namelist())
        if win_names != {f"{name}.exe" for name in BINS} | set(DOCS):
            raise SystemExit(f"zip contents {win_names}")
        try:
            package(root, "x86_64-unknown-linux-gnu", bin_dir, out, "0.0.1")
        except SystemExit as err:
            if "does not match" not in str(err):
                raise
        else:
            raise SystemExit("mismatched version was accepted")
    print("package self-test ok")


def packed_mode(archive: Path, name: str) -> int:
    with tarfile.open(archive, "r:gz") as packed:
        info = packed.getmember(name)
        return info.mode


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--target")
    parser.add_argument("--bin-dir", type=Path)
    parser.add_argument("--out", type=Path, default=Path("dist"))
    parser.add_argument("--version")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if not args.target or args.bin_dir is None:
        raise SystemExit("--target and --bin-dir are required")
    package(args.root, args.target, args.bin_dir, args.out, args.version)


if __name__ == "__main__":
    main()
