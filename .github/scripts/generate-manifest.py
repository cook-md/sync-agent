#!/usr/bin/env python3
import json
import os
import glob
from datetime import datetime

version = os.environ.get("VERSION")

def get_file_info(pattern):
    files = glob.glob(f"release-assets/{pattern}")
    if files:
        file_path = files[0]
        sha_file = f"{file_path}.sha256"
        if os.path.exists(sha_file):
            with open(sha_file, 'r') as f:
                sha256 = f.read().split()[0]
            size = os.path.getsize(file_path)
            filename = os.path.basename(file_path)
            return {
                "url": f"https://downloads.cook.md/sync-agent/v{version}/{filename}",
                "sha256": sha256,
                "size": size,
                "package_type": filename.split('.')[-1].lower()
            }
    return None

manifest = {
    "version": version,
    "notes": "See release notes on GitHub",
    "pub_date": datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ"),
    "platforms": {}
}

# Add macOS package
macos_dmg = get_file_info("CookSync-*.dmg")
if macos_dmg:
    manifest["platforms"]["macos-universal"] = macos_dmg

# Add Windows packages
win_x64 = get_file_info("CookSync-*-windows-x86_64.msi")
if win_x64:
    manifest["platforms"]["windows-x86_64"] = win_x64

# Add Linux packages
linux_packages = {}
deb = get_file_info("cook-sync_*_amd64.deb")
if deb:
    linux_packages["deb"] = deb
rpm = get_file_info("cook-sync-*.x86_64.rpm")
if rpm:
    linux_packages["rpm"] = rpm
appimage = get_file_info("CookSync-*.AppImage")
if appimage:
    linux_packages["appimage"] = appimage

if linux_packages:
    manifest["platforms"]["linux-x86_64"] = linux_packages

with open("latest.json", "w") as f:
    json.dump(manifest, f, indent=2)

print("✅ Manifest generated (no signature)")
