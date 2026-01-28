# Kuiper

Kuiper is a fast, multi-model AI CLI for orchestrating LLM tasks. It routes tasks between Gemini, Claude, and Codex, with a fast-path for simple prompts and full orchestration for complex tasks.

## Features

- **Smart Routing**: Automatically selects fast-path or full orchestration based on prompt complexity
- **Parallel Execution**: Run all models simultaneously and aggregate results
- **Auto-Tuning**: Learns from telemetry to optimize routing decisions
- **Response Caching**: Cache responses to reduce API costs
- **Retry with Backoff**: Automatic retries with exponential backoff
- **Streaming Output**: Real-time token streaming for responsive UX
- **Cross-Platform**: Native binaries for macOS (Intel/ARM) and Linux

## Installation

```bash
# Quick install (macOS/Linux)
curl -fsSL https://get.kuiper.dev | sh

# Homebrew (macOS)
brew tap ananda-callhub/kuiper
brew install kuiper

# From source
cargo install --path .
```

## Usage

```bash
# Basic usage (auto-routes to fast or full path)
kuiper do "convert json to yaml"

# Force fast-path (single model, no planning)
kuiper do "quick question" --fast

# Force full orchestration
kuiper do "design auth system" --full

# Parallel multi-model execution
kuiper do "explain quantum computing" --parallel

# Use research/complex-tier models
kuiper do "design distributed system" --research

# Use a specific model
kuiper do "write a haiku" --model claude-3-5-haiku-20241022

# Set budget limit
kuiper do "complex task" --budget 0.50

# List available models
kuiper models                    # Show all models
kuiper models --provider claude  # Show Claude models only
kuiper models --fast             # Show fast-tier models
kuiper models --complex          # Show complex-tier models

# Other commands
kuiper retry                  # Re-run last task
kuiper escalate "prompt"      # Force escalation
kuiper config                 # Show configuration
kuiper cache stats            # View cache statistics
kuiper cache clear            # Clear response cache
kuiper tuning show            # View auto-tuning parameters
kuiper tuning update          # Update from telemetry
```

## Configuration

Create `~/.kuiper/config.toml`:

```toml
default_budget = 0.25
fast_path = true
streaming = true

[fast_path_settings]
max_word_count = 25
escalation_threshold = 0.15

[models]
gemini = true
claude = true
codex = true

[models.gemini_settings]
model = "gemini-1.5-flash"
max_tokens = 8192

[models.claude_settings]
model = "claude-sonnet-4-5-20250929"
max_tokens = 8192

[models.codex_settings]
model = "gpt-4o"
max_tokens = 16384
```

## Environment Variables

```bash
export GEMINI_API_KEY="your-key"
export ANTHROPIC_API_KEY="your-key"
export OPENAI_API_KEY="your-key"
```

## Architecture

```
┌─────────────┐
│   CLI       │
└──────┬──────┘
       │
┌──────▼──────┐
│  Router     │─── Fast Path ──► Gemini/Codex (single model)
└──────┬──────┘
       │
┌──────▼──────┐     ┌─────────┐
│ Controller  │────►│ Cache   │
└──────┬──────┘     └─────────┘
       │
┌──────▼──────┐
│Orchestrator │─── Parallel ──► All models + aggregation
└──────┬──────┘
       │
┌──────▼──────┐     ┌─────────┐
│ Escalation  │────►│ Tuning  │
└─────────────┘     └─────────┘
```

### Model Tiers

Kuiper organizes models into three tiers for intelligent routing:

| Tier | Purpose | Models |
|------|---------|--------|
| **Fast** ⚡ | Quick, simple tasks | `gemini-1.5-flash`, `claude-3-5-haiku-20241022`, `gpt-4o-mini` |
| **Balanced** ⚖️ | General use | `gemini-1.5-pro`, `claude-sonnet-4-5-20250929`, `gpt-4o`, `gpt-4-turbo`, `o1-mini` |
| **Complex** 🧠 | Research & reasoning | `gemini-2.0-flash-exp`, `claude-opus-4-5-20251124`, `o1-preview` |

**Automatic tier selection:**
- `--fast` → Uses fast-tier models
- `--research` → Uses complex-tier models
- Default → Uses balanced-tier models from config

**Manual model selection:**
```bash
kuiper do --model gpt-4o "your prompt"
kuiper do --model o1-preview "complex coding task"
kuiper do --model claude-opus-4-5-20251124 "complex analysis"
kuiper do --model gemini-1.5-pro "large context task"
```

### Routing Logic

**Fast Path** (single model, no planning):
- Prompts under 25 words (tunable)
- No complex keywords (design, architecture, compare, optimize, etc.)
- Uses fast-tier models for efficiency
- Code-like prompts → Codex
- General prompts → Gemini

**Full Orchestration** (multi-step planning):
- Complex tasks requiring reasoning
- Uses complex-tier models for better results
- Codex creates execution plan
- Each step routed to best model

**Research Mode** (`--research`):
- Uses complex-tier models (o1, claude-opus-4, gemini-1.5-pro)
- Best for tasks requiring deep reasoning
- Higher cost but more capable

**Parallel Mode** (`--parallel`):
- Runs all available models simultaneously
- Calculates consensus score (Jaccard similarity)
- Returns best response based on quality heuristics

### Automatic Fallback

Kuiper has two levels of automatic fallback when models fail:

#### 1. Tiered Fallback (within same provider)
When a complex model's quota is exhausted, Kuiper tries lighter models from the same provider first:

```
Complex Model Quota Exceeded
        │
        ▼
┌─────────────────────────────────┐
│ Same Provider Fallback          │
│ gpt-5.2 → gpt-4.1 → gpt-4.1-nano│
│   or                            │
│ claude-opus → sonnet → haiku    │
│   or                            │
│ gemini-2.5-pro → flash → lite   │
└─────────────────────────────────┘
        │
        ▼
    Lighter Model (same provider)
```

#### 2. Provider Fallback (cross-provider)
If all models from one provider fail, Kuiper switches to another provider:

```
All Provider Models Failed
        │
        ▼
┌───────────────────┐
│ Provider Chain    │
│ OpenAI → Claude → Gemini
│   or              │
│ Claude → Gemini → OpenAI
│   or              │
│ Gemini → Claude → OpenAI
└───────────────────┘
        │
        ▼
    Different Provider
```

**Fallback triggers:**
- Rate limit exceeded (429)
- Quota exhausted
- Service unavailable (503)
- Timeout / connection errors
- Authentication errors

**Example (tiered fallback):**
```bash
# Research mode: if gpt-5.2 quota exceeded, falls back to gpt-4.1, then gpt-4.1-nano
kuiper do --research "analyze this complex problem"
[Research Mode] Primary model: gpt-5.2 (Complex tier)
[Research Mode] Fallback chain: gpt-5.2(Complex) → gpt-5.1(Complex) → gpt-4.1(Balanced) → gpt-4.1-nano(Fast)
[Fallback] gpt-5.2 failed (quota exceeded), trying next...
[Fallback] Trying gpt-5.1 (complex tier)...
[Fallback] gpt-5.1 failed (quota exceeded), trying next...
[Fallback] Trying gpt-4.1 (balanced tier)...
[Research Mode] Fell back from gpt-5.2, gpt-5.1 to gpt-4.1 (lighter model)
```

### Escalation Strategy

When a model response indicates uncertainty:
1. **Attempt 0**: Retry with different model
2. **Attempt 1**: Escalate to full orchestration
3. **Attempt 2+**: Request user clarification

### Auto-Tuning

Kuiper learns from your usage patterns:
- Adjusts fast-path word threshold based on escalation rate
- Updates model preference scores based on success/failure
- Run `kuiper tuning show` to see current parameters

## Project Structure

```
kuiper/
├── Cargo.toml
├── README.md
├── LICENSE
├── install.sh
├── .github/
│   └── workflows/
│       ├── ci.yml              # Tests, formatting, linting
│       └── release.yml         # Cross-platform builds + Homebrew
├── config/
│   └── defaults.toml
└── src/
    ├── main.rs                 # Entry point
    ├── cli.rs                  # CLI argument parsing
    ├── config.rs               # Configuration loading
    ├── controller.rs           # Task planning & execution
    ├── orchestrator.rs         # Parallel multi-model execution
    ├── fallback.rs             # Automatic model fallback on failures
    ├── cache.rs                # Response caching
    ├── retry.rs                # Retry with exponential backoff
    ├── tuning.rs               # Auto-tuning from telemetry
    ├── escalation.rs           # Uncertainty detection
    ├── streaming.rs            # Token streaming
    ├── telemetry.rs            # JSONL logging
    ├── routing/
    │   ├── mod.rs
    │   ├── fast_path.rs        # Fast-path routing
    │   └── heuristics.rs       # Routing heuristics
    └── models/
        ├── mod.rs              # Model dispatch
        ├── registry.rs         # Model registry & tier management
        ├── gemini.rs           # Google Gemini API
        ├── claude.rs           # Anthropic Claude API
        └── codex.rs            # OpenAI API
```

## Development

### Prerequisites

- Rust 1.70+ ([install](https://rustup.rs))

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

### Cross-Platform Build

```bash
# Install cross
cargo install cross

# Build for Linux
cross build --release --target x86_64-unknown-linux-gnu

# Build for macOS Intel
cross build --release --target x86_64-apple-darwin

# Build for macOS ARM
cross build --release --target aarch64-apple-darwin
```

### Adding a New Model

1. Create `src/models/newmodel.rs`
2. Implement `run()` and `stream()` functions
3. Add to `src/models/mod.rs`
4. Add config section in `src/config.rs`
5. Update routing heuristics

## Data Storage

Kuiper stores data in `~/.kuiper/`:

| File | Purpose |
|------|---------|
| `config.toml` | User configuration |
| `history.txt` | Prompt history for `retry` |
| `telemetry.jsonl` | JSONL usage logs |
| `cache.json` | Response cache |
| `tuning.json` | Auto-tuning parameters |

## Releasing

1. Update version in `Cargo.toml`
2. Commit and push
3. Create tag: `git tag v0.1.0 && git push --tags`
4. GitHub Actions will:
   - Build binaries for all platforms
   - Create GitHub Release
   - Update Homebrew formula

## License

MIT
