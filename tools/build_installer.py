"""Builds the Windows installer (calculator-windows-setup.exe) with NSIS.

Run from the project root:  python tools/build_installer.py
Output: target/dist/calculator-windows-setup.exe

Finds makensis.exe on PATH, in the normal NSIS install folders, or in the
portable copy Tauri downloads to %LOCALAPPDATA%\\tauri\\NSIS.
"""
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DIST = ROOT / "target" / "dist"


def find_makensis() -> Path:
    found = shutil.which("makensis")
    if found:
        return Path(found)
    candidates = [
        Path(os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)")) / "NSIS" / "makensis.exe",
        Path(os.environ.get("ProgramFiles", r"C:\Program Files")) / "NSIS" / "makensis.exe",
        Path(os.environ.get("LOCALAPPDATA", "")) / "tauri" / "NSIS" / "makensis.exe",
    ]
    for c in candidates:
        if c.exists():
            return c
    sys.exit("makensis.exe not found: install NSIS (winget install NSIS.NSIS)")


def app_version() -> str:
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    return re.search(r'^version\s*=\s*"([^"]+)"', text, re.M).group(1)


def run(cmd, **kw):
    print(">", " ".join(str(c) for c in cmd))
    subprocess.run([str(c) for c in cmd], check=True, **kw)


def main():
    makensis = find_makensis()
    version = app_version()
    print(f"NSIS: {makensis}\nversion: {version}")

    run(["cargo", "build", "--release", "--bin", "calculator"], cwd=ROOT)

    DIST.mkdir(parents=True, exist_ok=True)
    out = DIST / "calculator-windows-setup.exe"
    exe = ROOT / "target" / "release" / "calculator.exe"
    run([makensis, "/V2",
         f"/DVERSION={version}", f"/DEXE={exe}", f"/DOUTFILE={out}",
         ROOT / "installer" / "calculator.nsi"])
    print(f"\nInstaller ready: {out}  ({out.stat().st_size / 1e6:.1f} MB)")


if __name__ == "__main__":
    main()
