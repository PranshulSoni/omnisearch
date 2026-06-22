# Graph Report - .  (2026-06-23)

## Corpus Check
- 9 files · ~44,142 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 73 nodes · 112 edges · 14 communities detected
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## God Nodes (most connected - your core abstractions)
1. `wnd_proc()` - 13 edges
2. `reposition()` - 5 edges
3. `tick()` - 5 edges
4. `paint()` - 5 edges
5. `SearchEngine` - 5 edges
6. `get_app_icon()` - 4 edges
7. `main()` - 3 edges
8. `copy_directml()` - 3 edges
9. `State` - 3 edges
10. `run()` - 3 edges

## Surprising Connections (you probably didn't know these)
- `wnd_proc()` --calls--> `do_show()`  [EXTRACTED]
  opensearch-os\src\main.rs → opensearch-os\src\main.rs  _Bridges community 4 → community 6_
- `wnd_proc()` --calls--> `do_hide()`  [EXTRACTED]
  opensearch-os\src\main.rs → opensearch-os\src\main.rs  _Bridges community 4 → community 9_
- `wnd_proc()` --calls--> `get_app_icon()`  [EXTRACTED]
  opensearch-os\src\main.rs → opensearch-os\src\main.rs  _Bridges community 4 → community 1_
- `wnd_proc()` --calls--> `paint()`  [EXTRACTED]
  opensearch-os\src\main.rs → opensearch-os\src\main.rs  _Bridges community 4 → community 10_
- `tick()` --calls--> `reposition()`  [EXTRACTED]
  opensearch-os\src\main.rs → opensearch-os\src\main.rs  _Bridges community 6 → community 9_

## Communities

### Community 0 - "Community 0"
Cohesion: 0.2
Nodes (9): AnchorCategory, AppInfo, CatalogEntry, get_live_results(), get_local_ip(), MEMORYSTATUSEX, MetaJson, SearchResult (+1 more)

### Community 1 - "Community 1"
Cohesion: 0.33
Nodes (7): Anim, get_app_icon(), load_icon_from_memory(), main(), resolve_lnk(), run(), test_antigravity_icons()

### Community 2 - "Community 2"
Cohesion: 0.39
Nodes (5): mean_pool_norm(), scan_apps(), SearchEngine, test_hybrid_search_accuracy(), url_encode()

### Community 3 - "Community 3"
Cohesion: 0.43
Nodes (6): benchmark_model(), load_catalog(), main(), Benchmark multiple small embedding models against the Windows settings catalog., Return 1 if any keyword appears in control_name, breadcrumb_path, or synonyms., score_result()

### Community 4 - "Community 4"
Cohesion: 0.33
Nodes (6): copy_to_clipboard(), kick_debounce(), paste_from_clipboard(), SendHwnd, start_hide(), wnd_proc()

### Community 5 - "Community 5"
Cohesion: 0.7
Nodes (4): copy_directml(), copy_model(), find_directml(), main()

### Community 6 - "Community 6"
Cohesion: 0.4
Nodes (3): do_show(), reposition(), State

### Community 7 - "Community 7"
Cohesion: 0.83
Nodes (3): convert_to_ico(), generate_search_png(), main()

### Community 8 - "Community 8"
Cohesion: 0.67
Nodes (3): load_catalog(), main(), Build-time embedding pipeline. Run once to produce catalog.bin. Usage: python sc

### Community 9 - "Community 9"
Cohesion: 0.67
Nodes (3): do_hide(), ease_out(), tick()

### Community 10 - "Community 10"
Cohesion: 1.0
Nodes (3): badge(), fill(), paint()

### Community 11 - "Community 11"
Cohesion: 1.0
Nodes (2): main(), test_ico_load()

### Community 12 - "Community 12"
Cohesion: 1.0
Nodes (0): 

### Community 13 - "Community 13"
Cohesion: 1.0
Nodes (0): 

## Knowledge Gaps
- **11 isolated node(s):** `Anim`, `CatalogEntry`, `SearchResult`, `MetaJson`, `AnchorCategory` (+6 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `Community 12`** (2 nodes): `launcher.rs`, `launch()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 13`** (2 nodes): `export_bge_base.py`, `main()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `SearchEngine` connect `Community 2` to `Community 0`?**
  _High betweenness centrality (0.029) - this node is a cross-community bridge._
- **Why does `wnd_proc()` connect `Community 4` to `Community 1`, `Community 10`, `Community 6`, `Community 9`?**
  _High betweenness centrality (0.022) - this node is a cross-community bridge._
- **Why does `State` connect `Community 6` to `Community 1`?**
  _High betweenness centrality (0.010) - this node is a cross-community bridge._
- **What connects `Anim`, `CatalogEntry`, `SearchResult` to the rest of the system?**
  _11 weakly-connected nodes found - possible documentation gaps or missing edges._