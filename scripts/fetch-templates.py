#!/usr/bin/env python3
"""
Download TFT champion data and icons from Riot Data Dragon.
Creates data/champions.json and data/templates/champions/*.png

Usage: python3 scripts/fetch-templates.py [--set SET_NUMBER]
"""
import json
import os
import re
import shutil
import sys
import urllib.request

DATA_DRAGON_BASE = "https://ddragon.leagueoflegends.com"
PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CHAMPIONS_JSON = os.path.join(PROJECT_ROOT, "data", "champions.json")
TEMPLATES_DIR = os.path.join(PROJECT_ROOT, "data", "templates", "champions")


def get_latest_version():
    url = f"{DATA_DRAGON_BASE}/api/versions.json"
    with urllib.request.urlopen(url) as resp:
        versions = json.loads(resp.read())
    return versions[0]


def fetch_tft_champions(version):
    url = f"{DATA_DRAGON_BASE}/cdn/{version}/data/en_US/tft-champion.json"
    with urllib.request.urlopen(url) as resp:
        data = json.loads(resp.read())
    return data.get("data", {})


def detect_current_set(raw_champions):
    """Find the highest TFT set number in the data."""
    max_set = 0
    for champ_id in raw_champions:
        m = re.search(r"TFTSet(\d+)", champ_id)
        if m:
            max_set = max(max_set, int(m.group(1)))
    return max_set


def download_icon(url, path):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    if os.path.exists(path):
        return True
    try:
        urllib.request.urlretrieve(url, path)
        return True
    except Exception as e:
        print(f"  WARN: Failed to download {url}: {e}", file=sys.stderr)
        return False


def main():
    # Parse optional --set argument
    target_set = None
    if "--set" in sys.argv:
        idx = sys.argv.index("--set")
        if idx + 1 < len(sys.argv):
            target_set = int(sys.argv[idx + 1])

    print("Fetching latest Data Dragon version...")
    version = get_latest_version()
    print(f"  Version: {version}")

    print("Fetching TFT champion data...")
    raw_champions = fetch_tft_champions(version)
    print(f"  Found {len(raw_champions)} total champions across all sets")

    if target_set is None:
        target_set = detect_current_set(raw_champions)
    set_prefix = f"TFTSet{target_set}"
    # Also match the short form like TFT15_
    short_prefix = f"TFT{target_set}_"
    print(f"  Filtering to Set {target_set} ({set_prefix})")

    # Clean templates directory
    if os.path.exists(TEMPLATES_DIR):
        shutil.rmtree(TEMPLATES_DIR)
    os.makedirs(TEMPLATES_DIR, exist_ok=True)

    champions = []
    downloaded = 0

    for champ_id, champ_data in sorted(raw_champions.items()):
        # Filter to current set only
        if set_prefix not in champ_id and short_prefix not in champ_id:
            continue

        name = champ_data.get("name", champ_id)
        cost = champ_data.get("tier", 1)
        traits = [t.get("name", t.get("id", "")) for t in champ_data.get("traits", [])]
        icon_dd_file = champ_data.get("image", {}).get("full", f"{champ_id}.png")

        # Extract short ID (e.g. TFT15_Ahri from the full path)
        m = re.search(r"(TFT\d+_\w+)", champ_id)
        short_id = m.group(1) if m else champ_id.split("/")[-1]

        champions.append({
            "id": short_id,
            "name": name,
            "cost": cost,
            "traits": traits,
            "icon": f"{short_id}.png",
        })

        # Download icon
        url = f"{DATA_DRAGON_BASE}/cdn/{version}/img/tft-champion/{icon_dd_file}"
        path = os.path.join(TEMPLATES_DIR, f"{short_id}.png")
        if download_icon(url, path):
            downloaded += 1

    # Save champion data
    os.makedirs(os.path.dirname(CHAMPIONS_JSON), exist_ok=True)
    output = {
        "version": version,
        "set": target_set,
        "champions": champions,
    }
    with open(CHAMPIONS_JSON, "w") as f:
        json.dump(output, f, indent=2)

    print(f"\nDone!")
    print(f"  Set {target_set}: {len(champions)} champions saved to {CHAMPIONS_JSON}")
    print(f"  Icons: {downloaded} downloaded to {TEMPLATES_DIR}")


if __name__ == "__main__":
    main()
