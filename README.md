<img width="2000" height="500" alt="Banner" src="./images/Banner.png" />



# Mia Agent
![Build Status](https://github.com/mastermach50/mia-agent/actions/workflows/draft-release.yml/badge.svg)

Mia is a coding and personal assistant AI agent designed to be unobtrusive, configurable and easy to use. Mia grows _with_ you.

<img src="./images/mia.gif" height=500em>

This project was inspired by [Hermes Agent](https://hermes-agent.nousresearch.com/). Although it borrows ideas from it, this is not a direct derivation or modification of the project.

## Table of Contents   
- [Principles](#principles)          
- [Features](#features)  
- [Installation](#installation)
    - [Option A: Package Manager](#option-a-package-manager)
    - [Option B: Using cargo (Windows/Linux)](#option-b-using-cargo-windowslinux)
    - [Option C: Using nix flakes (NixOS/Linux)](#option-c-using-nix-flakes-nixoslinux)
- [Configuration](#configuration)
- [Usage](#usage)
- [Configuration Options](#configuration-options)
- [Development](#development)
- [Contributing](#contributing)

## Principles
- The agent should never write to the system without getting permission from the user.
- Guardrails should be implemented in code, not in prompts.
- The agent should act as an additional tool to the user not as a replacement for everything they already use.

## Features
- Minimal terminal UI powered by [ratatui](https://github.com/ratatui-org/ratatui)
- Multi turn agent loop
- Agent memory (persistent across sessions)
- Session and prompt history
- Full markdown rendering in terminal (thanks to [termimad](https://github.com/Canop/termimad))
- OpenAI Compatible API access with support for multiple providers:
  - OpenRouter (default, recommended)
  - Google AI Studio
  - Groq
  - Cerebras
  - Local LLMs (Ollama, LMStudio, llama.cpp, etc.)
- Web search and extract using [Tavily](https://tavily.com)
- Document conversion using [pandoc](https://pandoc.org) (`doc_convert` tool)
- Experimental streaming support
- Legacy reedline-based TUI available via `mia old-tui`

## Tools
Currently Mia has the following tools:

| Tool | Description | Gated |
|:-:|:-|:-:|
| 📅 | `datetime` | Get the current date and time | No |
| 📁 | `fs_list_dir` | List files in a directory | No |
| 📖 | `fs_read_file` | Read a file from the filesystem | No |
| 🔍 | `fs_grep_files` | Search file contents | No |
| 🧭 | `fs_search_dirs` | Search for files in a directory | No |
| ✍️ | `fs_write_file` | Write content to a file | Yes |
| 🐍 | `exec_python` | Execute Python 3 code | Yes |
| 🐚 | `exec_shell` | Run shell commands | Yes |
| 🧠 | `memory` | Manage memory about the user and the agent | No |
| 🪏 | `web_extract` | Extract content from a URL | No |
| 🌐 | `web_search` | Search the web | No |
| 📄 | `doc_convert` | Convert documents using pandoc | No |

## Installation
### Option A: Package Manager
#### winget (Windows)
```
winget install Mach50.MiaAgent
```
Open a new terminal and run
```
mia setup
```
to configure your agent and start using it.
### Option B: Using cargo (Windows/Linux)
Mia requires rust nightly
```
rustup toolchain install nightly
rustup default nightly
```
Cargo can fetch, build and install mia.
```
cargo +nightly install --git https://github.com/mastermach50/mia-agent
```
Run
```
mia setup
```
to configure your agent and start using it.
### Option C: Using nix flakes (NixOS/Linux)
Run the given command to test out mia on your system.
```
nix run github:mastermach50/mia-agent
```
To install it, add to your system packages using the same flake.

## Configuration
Mia supports multiple LLM providers:

- **[OpenRouter](https://openrouter.ai/workspaces/default/keys)** (default, recommended) - can be obtained for free, [there are also free models](https://openrouter.ai/docs/guides/routing/model-variants/free)
- **[Google AI Studio](https://aistudio.google.com/)** - free tier available
- **[Groq](https://groq.com/)** - fast inference API
- **[Cerebras](https://www.cerebras.ai/)** - large model inference
- **Local LLMs** - Ollama, LMStudio, llama.cpp, and other OpenAI-compatible local servers

1. Run `mia setup` to configure your agent interactively.

2. For advanced configuration, you can manually edit the `~/.mia/.env` file on Linux or `C:\Users\<username>\.mia\.env` on Windows to add your API key:
```
OPENROUTER_API_KEY=<your-openrouter-api-key>
```
Or for other providers, set the appropriate environment variable.

3. (Optional) Add your [Tavily API key](https://app.tavily.com/home) (can be obtained for free and has a reasonable number of free searches per month) to `.mia/.env` to enable the `web_search` and `web_extract` tools.
```
TAVILY_API_KEY=<your-tavily-api-key>
```

4. (Optional) Run `mia tools` to see if all the tools are usable and find which components are missing if any.

Currently the external tools required by mia are
- [Python](https://python.org)
- [Ripgrep](https://github.com/BurntSushi/ripgrep)
- [fd](https://github.com/sharkdp/fd)

## Usage
### CLI
Use `mia --help` to access the full cli help menu.

### TUI
The TUI supports certain commands. Use `/help` while in the TUI to see them.

### Available Commands
- `mia tui` - Start the main terminal UI (ratatui-based)
- `mia old-tui` - Start the legacy reedline-based TUI (hidden command)
- `mia setup` - Interactive setup wizard
- `mia tools` - Check tool availability
- `mia model list` - List available models
- `mia model list --free` - List only free models (price filter)
- `mia model list --min-context <number>` - Filter by minimum context length
- `mia model list --max-price <price>` - Filter by maximum price per million tokens
- `mia model show` - Show current model info
- `mia sessions list` - List all sessions
- `mia session clear` - Clear all sessions (prompts for confirmation)

## Configuration Options

The `~/.mia/config.toml` file contains the following options:

```toml
[model]
provider = "openrouter"        # LLM provider: openrouter, google, groq, cerebras, local
base_url = "https://openrouter.ai/api/v1"  # API base URL
name = "openrouter/owl-alpha"  # Model name
reasoning = "medium"           # Reasoning effort: low, medium, high

[agent]
max_iterations = 20            # Maximum agent iterations per request

[tui]
username = "user"              # Display name in TUI
max_history = 1000             # Maximum conversation history
show_reasoning = true           # Show reasoning output
streaming = true               # Enable streaming responses
show_spinner = true             # Show spinner during API calls
```

## Development
### MSRV
Mia uses rust nightly as it is required for `whatsapp-rust` and certain necessary features.

## Contributing
We DO NOT accept vibecoded contributions or contributions from agents.

However AI assisted code is fine as long as you the human are the one contributing.

Human contributions are always welcome.