# Graph Report - .  (2026-06-23)

## Corpus Check
- 10 files · ~49,156 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 97 nodes · 159 edges · 17 communities detected
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## God Nodes (most connected - your core abstractions)
1. `wnd_proc()` - 12 edges
2. `trigger_icon_loading()` - 6 edges
3. `SearchEngine` - 6 edges
4. `reposition()` - 5 edges
5. `tick()` - 5 edges
6. `paint()` - 5 edges
7. `try_calc()` - 5 edges
8. `run_indexer()` - 4 edges
9. `do_show()` - 4 edges
10. `get_app_icon()` - 4 edges

## Surprising Connections (you probably didn't know these)
- `wnd_proc()` --calls--> `trigger_icon_loading()`  [EXTRACTED]
  opensearch-os\src\main.rs → opensearch-os\src\main.rs  _Bridges community 2 → community 1_
- `wnd_proc()` --calls--> `paint()`  [EXTRACTED]
  opensearch-os\src\main.rs → opensearch-os\src\main.rs  _Bridges community 2 → community 5_
- `try_calc()` --calls--> `parse_expr()`  [EXTRACTED]
  opensearch-os\src\search.rs → opensearch-os\src\search.rs  _Bridges community 12 → community 8_

## Communities

### Community 0 - "Community 0"
Cohesion: 0.14
Nodes (12): AnchorCategory, AppInfo, CatalogEntry, mean_pool_norm(), MEMORYSTATUSEX, MetaJson, QuickAction, RecentFileInfo (+4 more)

### Community 1 - "Community 1"
Cohesion: 0.27
Nodes (10): Anim, get_app_icon(), get_file_icon(), load_icon_from_memory(), main(), resolve_lnk(), run(), SendHwnd (+2 more)

### Community 2 - "Community 2"
Cohesion: 0.27
Nodes (10): copy_to_clipboard(), do_hide(), do_show(), ease_out(), kick_debounce(), paste_from_clipboard(), reposition(), start_hide() (+2 more)

### Community 3 - "Community 3"
Cohesion: 0.47
Nodes (4): get_quick_actions(), SearchEngine, test_hybrid_search_accuracy(), url_encode()

### Community 4 - "Community 4"
Cohesion: 0.43
Nodes (6): benchmark_model(), load_catalog(), main(), Benchmark multiple small embedding models against the Windows settings catalog., Return 1 if any keyword appears in control_name, breadcrumb_path, or synonyms., score_result()

### Community 5 - "Community 5"
Cohesion: 0.4
Nodes (4): badge(), fill(), paint(), State

### Community 6 - "Community 6"
Cohesion: 0.7
Nodes (4): copy_directml(), copy_model(), find_directml(), main()

### Community 7 - "Community 7"
Cohesion: 0.7
Nodes (4): get_scan_folders(), read_text_file(), run_indexer(), start_indexer()

### Community 8 - "Community 8"
Cohesion: 0.5
Nodes (5): parse_expr(), parse_power(), parse_primary(), parse_term(), parse_unary()

### Community 9 - "Community 9"
Cohesion: 0.83
Nodes (3): get_known_folder_path(), handle_action(), launch()

### Community 10 - "Community 10"
Cohesion: 0.83
Nodes (3): convert_to_ico(), generate_search_png(), main()

### Community 11 - "Community 11"
Cohesion: 0.67
Nodes (3): load_catalog(), main(), Build-time embedding pipeline. Run once to produce catalog.bin. Usage: python sc

### Community 12 - "Community 12"
Cohesion: 0.67
Nodes (3): tokenize(), try_calc(), try_pct_of()

### Community 13 - "Community 13"
Cohesion: 1.0
Nodes (2): main(), test_ico_load()

### Community 14 - "Community 14"
Cohesion: 1.0
Nodes (2): get_live_results(), get_local_ip()

### Community 15 - "Community 15"
Cohesion: 1.0
Nodes (2): resolve_lnk_path(), scan_recent_files()

### Community 16 - "Community 16"
Cohesion: 1.0
Nodes (0): 

## Knowledge Gaps
- **14 isolated node(s):** `Anim`, `CatalogEntry`, `SearchResult`, `MetaJson`, `AnchorCategory` (+9 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `Community 14`** (2 nodes): `get_live_results()`, `get_local_ip()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 15`** (2 nodes): `resolve_lnk_path()`, `scan_recent_files()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 16`** (2 nodes): `export_bge_base.py`, `main()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `SearchEngine` connect `Community 3` to `Community 0`?**
  _High betweenness centrality (0.030) - this node is a cross-community bridge._
- **Why does `wnd_proc()` connect `Community 2` to `Community 1`, `Community 5`?**
  _High betweenness centrality (0.013) - this node is a cross-community bridge._
- **Why does `State` connect `Community 5` to `Community 1`?**
  _High betweenness centrality (0.009) - this node is a cross-community bridge._
- **What connects `Anim`, `CatalogEntry`, `SearchResult` to the rest of the system?**
  _14 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.14 - nodes in this community are weakly interconnected._