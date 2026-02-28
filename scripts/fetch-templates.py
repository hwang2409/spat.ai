#!/usr/bin/env python3
"""
Download champion and item icons from Riot Data Dragon.
Usage: python3 scripts/fetch-templates.py
"""
import json
import os
import urllib.request

DATA_DRAGON_BASE = "https://ddragon.leagueoflegends.com"


def get_latest_version():
    url = f"{DATA_DRAGON_BASE}/api/versions.json"
    with urllib.request.urlopen(url) as resp:
        versions = json.loads(resp.read())
    return versions[0]


def fetch_tft_champions(version):
    """Fetch TFT champion data from Data Dragon."""
    url = f"{DATA_DRAGON_BASE}/cdn/{version}/data/en_US/tft-champion.json"
    try:
        with urllib.request.urlopen(url) as resp:
            data = json.loads(resp.read())
        return data.get("data", {})
    except Exception as e:
        print(f"Warning: Could not fetch TFT champion data: {e}")
        return {}


def download_icon(url, path):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    if os.path.exists(path):
        print(f"  Skip (exists): {path}")
        return
    try:
        urllib.request.urlretrieve(url, path)
        print(f"  Downloaded: {path}")
    except Exception as e:
        print(f"  Failed: {path} - {e}")


def main():
    version = get_latest_version()
    print(f"Using Data Dragon version: {version}")

    champions = fetch_tft_champions(version)

    champ_dir = os.path.join("data", "templates", "champions")
    for champ_id, champ_data in champions.items():
        icon_name = champ_data.get("image", {}).get("full", f"{champ_id}.png")
        url = f"{DATA_DRAGON_BASE}/cdn/{version}/img/tft-champion/{icon_name}"
        path = os.path.join(champ_dir, f"{champ_id}.png")
        download_icon(url, path)

    print(f"\nDone! Downloaded {len(champions)} champion icons.")


if __name__ == "__main__":
    main()
