"""
Benchmark multiple small embedding models against the Windows settings catalog.
Scores each model on hit@3 for 50 natural-language queries.

Usage: python scripts/benchmark_embeddings.py
"""

import csv
import time
import os
import numpy as np
from pathlib import Path
from sentence_transformers import SentenceTransformer

CATALOG_PATH = Path(__file__).parent.parent / "settings_catalog.csv"

MODELS = [
    ("sentence-transformers/all-MiniLM-L6-v2", "MiniLM-L6", False),
]

# 50 queries a real user would type, with keywords to auto-score against
# keyword list = words that should appear in control_name or breadcrumb_path of a correct result
QUERIES = [
    ("stop mouse from jumping",               ["pointer precision", "pointer speed", "mouse"]),
    ("disable startup programs",              ["startup", "autostart"]),
    ("change time zone",                      ["time zone", "timezone"]),
    ("turn off notifications",                ["notification"]),
    ("fix blurry text",                       ["ccd cleartype", "cleartype", "dpi", "blurry", "scale"]),
    ("allow apps through firewall",           ["firewall"]),
    ("make text bigger",                      ["text size", "font size", "scale", "display"]),
    ("change display brightness",             ["brightness"]),
    ("connect to wifi",                       ["wi-fi", "wifi", "wireless"]),
    ("remove a printer",                      ["printer", "print"]),
    ("enable dark mode",                      ["dark", "color mode", "theme", "appearance"]),
    ("change screen resolution",              ["resolution", "display"]),
    ("set default browser",                   ["default app", "default browser", "browser"]),
    ("disable auto updates",                  ["update", "windows update"]),
    ("sleep settings",                        ["sleep", "power"]),
    ("change wallpaper",                      ["wallpaper", "background", "desktop background"]),
    ("enable bluetooth",                      ["bluetooth"]),
    ("disable touchpad",                      ["touchpad", "trackpad"]),
    ("configure microphone",                  ["microphone", "input device"]),
    ("change language",                       ["language", "region"]),
    ("set up fingerprint login",              ["fingerprint", "biometric", "windows hello"]),
    ("clear storage space",                   ["storage", "disk cleanup", "disk space"]),
    ("rename this computer",                  ["computer name", "rename pc", "device name"]),
    ("change sound output device",            ["sound output", "audio output", "speaker", "playback"]),
    ("reduce eye strain at night",            ["night light", "blue light", "color temperature"]),
    ("stop apps from running in background",  ["background app"]),
    ("speed up animations",                   ["animation", "visual effect", "transition"]),
    ("uninstall a program",                   ["uninstall", "remove app", "apps & features"]),
    ("disable cortana",                       ["cortana", "search"]),
    ("set proxy settings",                    ["proxy"]),
    ("change mouse speed",                    ["pointer speed", "mouse speed", "cursor speed"]),
    ("flip screen upside down",               ["rotation", "orientation", "display"]),
    ("enable remote desktop",                 ["remote desktop", "rdp"]),
    ("set up vpn",                            ["vpn", "virtual private"]),
    ("configure parental controls",           ["parental", "family safety", "child"]),
    ("map network drive",                     ["network drive", "map drive"]),
    ("change power plan",                     ["power plan", "battery saver", "performance"]),
    ("set up email account",                  ["email", "mail", "account"]),
    ("configure taskbar",                     ["taskbar"]),
    ("disable location services",             ["location"]),
    ("change keyboard layout",               ["keyboard layout", "input method", "language"]),
    ("enable magnifier",                      ["magnifier"]),
    ("set up multiple monitors",              ["multiple display", "second screen", "extend"]),
    ("change user account picture",           ["account picture", "profile picture", "user photo"]),
    ("disable password requirement",          ["password", "sign-in", "sign in option"]),
    ("configure storage sense",               ["storage sense"]),
    ("enable developer mode",                 ["developer mode"]),
    ("sync settings between devices",         ["sync", "backup", "cloud"]),
    ("change default search engine",          ["search", "default search"]),
]


def load_catalog():
    rows = []
    with open(CATALOG_PATH, encoding="utf-8") as f:
        for row in csv.DictReader(f):
            synonyms = row["synonyms"].replace("|", " ")
            text = f"{row['control_name']} {synonyms} {row['description']} {row['breadcrumb_path']}"
            rows.append({
                "text": text,
                "control_name": row["control_name"],
                "breadcrumb_path": row["breadcrumb_path"],
                "synonyms": row["synonyms"].replace("|", " "),
            })
    return rows


def score_result(result_rows, keywords):
    """Return 1 if any keyword appears in control_name, breadcrumb_path, or synonyms."""
    for r in result_rows:
        haystack = (r["control_name"] + " " + r["breadcrumb_path"] + " " + r["synonyms"]).lower()
        if any(kw.lower() in haystack for kw in keywords):
            return 1
    return 0


def benchmark_model(model_name, display_name, needs_prefix, catalog_rows):
    print(f"\n{'='*60}")
    print(f"Model: {display_name} ({model_name})")
    print(f"{'='*60}")

    model = SentenceTransformer(model_name)

    # Embed catalog
    texts = [r["text"] for r in catalog_rows]
    t0 = time.perf_counter()
    corpus_vecs = model.encode(texts, batch_size=64, normalize_embeddings=True, show_progress_bar=True)
    embed_time = time.perf_counter() - t0
    print(f"Catalog embedded in {embed_time:.1f}s  |  shape: {corpus_vecs.shape}")

    # Get model cache size
    cache_dir = Path.home() / ".cache" / "huggingface" / "hub"
    model_slug = model_name.replace("/", "--")
    model_path = list(cache_dir.glob(f"models--{model_slug}"))
    model_size_mb = 0
    if model_path:
        model_size_mb = sum(f.stat().st_size for f in model_path[0].rglob("*") if f.is_file()) / 1e6

    # Run queries
    hits = 0
    query_times = []
    misses = []

    for query_text, keywords in QUERIES:
        prefix = "query: " if needs_prefix else ""
        qt0 = time.perf_counter()
        qvec = model.encode([prefix + query_text], normalize_embeddings=True)[0]
        scores = corpus_vecs @ qvec
        top3_idx = np.argsort(scores)[-3:][::-1]
        query_times.append(time.perf_counter() - qt0)

        top3 = [catalog_rows[i] for i in top3_idx]
        hit = score_result(top3, keywords)
        hits += hit
        if not hit:
            misses.append((query_text, top3[0]["control_name"], top3[0]["breadcrumb_path"]))

    avg_query_ms = np.mean(query_times) * 1000
    hit_rate = hits / len(QUERIES) * 100

    print(f"\nHit@3: {hits}/{len(QUERIES)} = {hit_rate:.0f}%")
    print(f"Avg query latency: {avg_query_ms:.1f}ms")
    print(f"Model size (cache): {model_size_mb:.0f}MB")

    if misses:
        print(f"\nMissed queries ({len(misses)}):")
        for q, top_name, top_path in misses:
            print(f"  MISS '{q}'  -> got: '{top_name}' ({top_path})")

    return {
        "model": display_name,
        "hit_rate": hit_rate,
        "hits": hits,
        "avg_query_ms": avg_query_ms,
        "embed_time_s": embed_time,
        "size_mb": model_size_mb,
    }


def main():
    print("Loading catalog...")
    catalog_rows = load_catalog()
    print(f"Loaded {len(catalog_rows)} rows")

    results = []
    for model_name, display_name, needs_prefix in MODELS:
        r = benchmark_model(model_name, display_name, needs_prefix, catalog_rows)
        results.append(r)

    print(f"\n{'='*60}")
    print("SUMMARY")
    print(f"{'='*60}")
    print(f"{'Model':<14} {'Hit@3':>6} {'Score':>7} {'Query ms':>10} {'Size MB':>9}")
    print(f"{'-'*50}")
    results.sort(key=lambda x: x["hit_rate"], reverse=True)
    for r in results:
        print(f"{r['model']:<14} {r['hits']:>3}/50 {r['hit_rate']:>6.0f}%  {r['avg_query_ms']:>8.1f}ms  {r['size_mb']:>7.0f}MB")


if __name__ == "__main__":
    main()
