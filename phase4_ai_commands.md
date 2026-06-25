# Phase 4 — AI Commands, Agents & Smart Search

Design document for `opensearch-os` — compiled from [raycast_features.md](file:///c:/Users/Pranshul%20Soni/Documents/Projects/Backend/Project-Raycast/raycast_features.md) and all conversation context.

---

## 🧠 AI Model Strategy

> *"Wherever I am saying using AI I mean we are gonna give user DeepSeek V4 Flash API key for free, else they can choose their model."*

| Tier | Model | Cost | Who Gets It |
|---|---|---|---|
| **Default (Free)** | DeepSeek V4 Flash | We provide a shared free key | Everyone, zero setup |
| **Bring Your Own** | Any OpenAI-compatible model | User's own key | Power users |
| **Supported BYO** | OpenAI, Groq, Ollama (local), Claude, Gemini | — | Via "Manage Models" command |

---

## 1. AI Commands (Full List from raycast_features.md)

### 1.1 Core AI Actions

| Command | Trigger | What It Does |
|---|---|---|
| **AI Chat** | `chat <message>` | Inline multi-turn conversation with the selected model |
| **Ask Clipboard** | `ask clipboard` | Sends current clipboard text to AI and shows answer inline |
| **Ask File Explorer** | `ask file <path>` | Reads a file and lets you ask questions about its content |
| **Ask Web** | `ask web <url>` | Fetches a webpage and answers questions about it |
| **Explain This** | `explain` | Explains clipboard content in simple terms |
| **Find Bugs in Code** | `find bugs` | Sends clipboard code to AI for bug detection |
| **Fix Spelling & Grammar** | `fix grammar` | Fixes clipboard text grammar/spelling, auto-pastes result |
| **Summarize Webpage** | `summarize <url>` | Fetches and summarizes a webpage inline |
| **Send Selected Text to AI** | `send text` | Takes selected text from active app + opens chat |
| **Translate** | `translate <text>` | Translates text via AI (any language) |
| **Show Memory** | `memory` | Shows what the AI remembers about you across sessions |
| **Create AI Command** | `create ai command` | Save a custom prompt as a reusable named command |
| **Search AI Commands** | `ai commands` | Browse all your saved custom AI prompts |
| **Manage Models** | `models` | Add/remove/switch AI model backends + API keys |
| **Profile** | `profile` | View/edit your AI profile (used to personalize responses) |

---

## 2. Hermes Agent System

> *"Need to integrate Hermes agent in our app such that user can create a particular task using Hermes agent using DeepSeek V4 Flash (free). It should actually create an agent in the background using Hermes and the user should be able to chat with it or give it tasks to do — using 'View Agents' and then viewing the agents."*

### What Is a Hermes Agent?

A **Hermes agent** is a persistent background AI process that:
- Has a name, a goal, and a system prompt
- Runs tasks autonomously (open files, search the web, write notes, run scripts)
- Can be chatted with like a person
- Persists in SQLite — survives launcher restarts
- Runs on DeepSeek V4 Flash (free key provided)

### Agent Commands

| Command | Trigger | What It Does |
|---|---|---|
| **Create Agent** | `create agent` | Opens a form: Name → Goal → System prompt → spawns agent |
| **Search Agents** | `agents:` or `search agents` | Browse all created agents |
| **Chat with Agent** | Click agent → Enter | Opens inline chat with that specific agent |
| **Give Agent a Task** | Type task in chat | Agent reasons and executes the task in background |
| **View Agent Status** | `agents:` | See which agents are running, idle, or completed |
| **Delete Agent** | Right-click agent → Delete | Stops and removes the agent |

### Architecture

```
User types: create agent
                ↓
Form: Name = "Research Bot"
      Goal = "Help me research any topic and save notes"
      Model = DeepSeek V4 Flash (default)
                ↓
Agent stored in SQLite: agents table
Background thread spawned: AgentRuntime
                ↓
User types: agents:
→ Shows "Research Bot" · [IDLE] · Last used: just now
                ↓
User clicks Research Bot → types: "Find me 3 papers on transformer attention"
→ Agent reasons, uses tools (web search, note creation), streams response
```

### Agent Tools Available to Hermes
- `web_search(query)` — search the web
- `read_clipboard()` — read clipboard content
- `write_note(title, content)` — create a note
- `open_url(url)` — launch a browser URL
- `run_command(cmd)` — execute a shell command
- `read_file(path)` — read a file's content

---

## 3. Workflows

> *"The basic idea is that the user can either create a workflow by writing a prompt to DeepSeek or can create manual workflows. Like when fired a particular search then it starts that particular workflow. E.g. Study Mode: enables firewall filters, turns on night light, opens study lofi — like that."*

### Workflow Types

| Type | How Created | Example |
|---|---|---|
| **AI-Generated** | Describe what you want in plain English → DeepSeek generates the steps | "When I'm studying, block distractions" |
| **Manual** | Pick actions from a list and chain them | Open Spotify → Set Volume 40% → Enable Night Light |
| **Trigger-Based** | Fires when a specific search term is typed | Typing "study mode" runs the workflow |
| **Scheduled** | Runs at a time (like a cron) | 9 AM every weekday: open email + calendar |

### Workflow Examples

```yaml
Name: Study Mode
Trigger: "study mode"
Steps:
  - system: enable_night_light
  - system: toggle_hidden_files off
  - launch: "https://open.spotify.com/playlist/study-lofi"
  - system: set_volume 35
  - focus: start_session duration=90min

Name: Morning Routine
Trigger: "morning"
Steps:
  - launch: "https://calendar.google.com"
  - launch: "https://mail.google.com"
  - system: set_volume 50
  - ai: summarize today's calendar events
```

### Workflow Commands

| Command | Trigger |
|---|---|
| Create Workflow | `create workflow` |
| Search Workflows | `workflows:` |
| Run Workflow | Type workflow name |
| Edit Workflow | Click workflow → Edit |

---

## 4. Browser AI Prefixes (Quick Wins)

These open an AI service in the browser with the prompt auto-submitted — **no extra Enter needed** on the website.

| Command | Opens |
|---|---|
| `chatgpt <prompt>` | `chatgpt.com/?q=<prompt>` ✅ **Already built** |
| `claude <prompt>` | `claude.ai/new?q=<prompt>` |
| `perplexity <prompt>` | `perplexity.ai/search?q=<prompt>` |
| `gemini <prompt>` | `gemini.google.com/app?q=<prompt>` |
| `deepseek <prompt>` | `chat.deepseek.com/?q=<prompt>` |
| `grok <prompt>` | `grok.com/?q=<prompt>` |

---

## 5. Image OCR Indexing

> *"Not the image name but rather the content inside the images/photos. Use OCR for that — if the Windows one is not good then we can use a very small OCR model or Windows OCR using API call."*

### Plan
- Walk the **Pictures folder** (user confirmed this is the priority)
- Use **Windows.Media.Ocr** API — built into Windows 10/11, fast (~100ms/image), offline, no download
- Store extracted OCR text in the existing `files_fts` SQLite table
- Images appear in `file:` search scope with OCR text snippet in the breadcrumb

### Example
```
User types: file: quarterly budget
Result: 📷  Screenshot_2024-03-15.png
         File > Pictures | "...Q1 quarterly budget projection..."
        → Enter opens image in default viewer
```

---

## 6. Additional Features from raycast_features.md (Not Yet Built)

### Focus Sessions
| Command | Status |
|---|---|
| Start Focus Session | ❌ Not yet |
| Toggle Focus Session | ❌ Not yet |
| Create Focus Category | ❌ Not yet |
| Search Focus Categories | ❌ Not yet |

### Notes
| Command | Status |
|---|---|
| Create Note | ❌ Not yet |
| Search Notes | ❌ Not yet |

### System Actions (Missing from Phase 3)
| Command | Status |
|---|---|
| Toggle System Appearance (Dark/Light) | ❌ Not yet |
| Toggle Bluetooth | ❌ Not yet |
| Show Screen Saver | ❌ Not yet |
| Toggle HDR | ❌ Not yet |
| Hide All Apps Except Frontmost | ❌ Not yet |
| Quit All Apps | ❌ Not yet |
| Log Out | ❌ Not yet |
| Paste Latest Screenshot | ❌ Not yet |
| Show Desktop | ❌ Not yet |
| Quick Look (file preview) | ❌ Not yet |

### Window Management (Missing from Phase 1)
| Command | Status |
|---|---|
| Full grid of Half/Third/Quarter snapping | ❌ Not yet |
| Move to Virtual Desktop (1–10) | ❌ Not yet |
| Toggle Always on Top | ❌ Not yet |
| Move to Next/Previous Display | ❌ Not yet |

### Web Searches
| Command | Status |
|---|---|
| Bing, DuckDuckGo prefixes | ❌ Not yet |
| Google Search prefix | ❌ Not yet |

### Search Screenshots
| Command | Status |
|---|---|
| Search Screenshots (OCR content) | ❌ Not yet (part of OCR phase) |

---

## 7. Implementation Sequence

```
Phase 4a — Browser AI Prefixes (1–2 hours, zero dependencies)
└── Add claude:, perplexity:, gemini:, deepseek:, grok: prefixes to search.rs

Phase 4b — Image OCR Indexing (2–3 days)
├── Add ocr_indexer.rs
├── Windows.Media.Ocr API integration
└── Store in files_fts, show in file: search

Phase 4c — Inline AI Chat + Core Commands (1 week)
├── Add reqwest + serde to Cargo.toml
├── DeepSeek V4 Flash API integration (free key)
├── Multi-line scrollable text rendering in paint()
├── Streaming token rendering
└── Core commands: ask, explain, find bugs, fix grammar, translate

Phase 4d — Hermes Agent System (1–2 weeks)
├── agents SQLite table
├── AgentRuntime background thread
├── Tool implementations (web search, notes, file read)
├── agents: folder + agent chat UI
└── Create Agent form

Phase 4e — Workflows (1 week)
├── workflows SQLite table
├── AI workflow generation via DeepSeek
├── Manual workflow builder
├── Trigger-based execution
└── create workflow + workflows: commands
```

---

## 8. Open Questions

> [!IMPORTANT]
> **DeepSeek Free Key**: How do we share the free key without abuse? Options:
> - Hardcode a rate-limited key (easiest, acceptable for personal use)
> - Proxy through a small serverless function so the key is never exposed in the binary

> [!IMPORTANT]
> **Hermes Framework**: Do we build the agent runtime from scratch using DeepSeek function-calling, or integrate an existing Rust agent library? Recommendation: build lightweight from scratch using DeepSeek's tool-use API (simpler, no extra deps).

> [!NOTE]
> **AI memory persistence**: "Show Memory" implies cross-session memory. This can be stored in a simple `ai_memory` SQLite table — key/value facts the AI learns about the user over time.
