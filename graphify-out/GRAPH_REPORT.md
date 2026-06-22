# Graph Report - .  (2026-06-23)

## Corpus Check
- 13 files · ~54,278 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 116 nodes · 194 edges · 14 communities detected
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## God Nodes (most connected - your core abstractions)
1. `wnd_proc()` - 12 edges
2. `SearchEngine` - 11 edges
3. `run_browser_indexer()` - 6 edges
4. `trigger_icon_loading()` - 6 edges
5. `reposition()` - 5 edges
6. `tick()` - 5 edges
7. `paint()` - 5 edges
8. `try_calc()` - 5 edges
9. `run_indexer()` - 4 edges
10. `run()` - 4 edges

## Surprising Connections (you probably didn't know these)
- None detected - all connections are within the same source files.

## Communities

### Community 0 - "Community 0"
Cohesion: 0.14
Nodes (25): Anim, badge(), copy_to_clipboard(), do_hide(), do_show(), ease_out(), fill(), get_app_icon() (+17 more)

### Community 1 - "Community 1"
Cohesion: 0.11
Nodes (24): AnchorCategory, AppInfo, CatalogEntry, get_live_results(), get_local_ip(), mean_pool_norm(), MEMORYSTATUSEX, MetaJson (+16 more)

### Community 2 - "Community 2"
Cohesion: 0.35
Nodes (4): get_quick_actions(), SearchEngine, test_hybrid_search_accuracy(), url_encode()

### Community 3 - "Community 3"
Cohesion: 0.46
Nodes (7): get_browser_profiles(), parse_bookmarks(), parse_firefox(), parse_history(), run_browser_indexer(), start_browser_indexer(), traverse_bookmarks()

### Community 4 - "Community 4"
Cohesion: 0.43
Nodes (6): benchmark_model(), load_catalog(), main(), Benchmark multiple small embedding models against the Windows settings catalog., Return 1 if any keyword appears in control_name, breadcrumb_path, or synonyms., score_result()

### Community 5 - "Community 5"
Cohesion: 0.7
Nodes (4): copy_directml(), copy_model(), find_directml(), main()

### Community 6 - "Community 6"
Cohesion: 0.7
Nodes (4): get_scan_folders(), read_text_file(), run_indexer(), start_indexer()

### Community 7 - "Community 7"
Cohesion: 0.83
Nodes (3): index_single_repo(), run_git_indexer(), start_git_indexer()

### Community 8 - "Community 8"
Cohesion: 0.83
Nodes (3): get_known_folder_path(), handle_action(), launch()

### Community 9 - "Community 9"
Cohesion: 0.83
Nodes (3): convert_to_ico(), generate_search_png(), main()

### Community 10 - "Community 10"
Cohesion: 0.67
Nodes (3): load_catalog(), main(), Build-time embedding pipeline. Run once to produce catalog.bin. Usage: python sc

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
- **14 isolated node(s):** `Anim`, `CatalogEntry`, `SearchResult`, `MetaJson`, `AnchorCategory` (+9 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `Community 12`** (2 nodes): `export_bge_base.py`, `main()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 13`** (1 nodes): `check_db.py`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `SearchEngine` connect `Community 2` to `Community 1`?**
  _High betweenness centrality (0.059) - this node is a cross-community bridge._
- **What connects `Anim`, `CatalogEntry`, `SearchResult` to the rest of the system?**
  _14 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.14 - nodes in this community are weakly interconnected._
- **Should `Community 1` be split into smaller, more focused modules?**
  _Cohesion score 0.11 - nodes in this community are weakly interconnected._