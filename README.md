# Mia Agent


Mia is a coding and personal assistant AI agent designed to be unobtrusive, configurable ,easy to use. Mia grows with you.

This project way inspired by [Hermes Agent](https://hermes-agent.nousresearch.com/). Although it borrows ideas from it, this is not a direct derivation or modification of the project.

## Table of Contents   
- [Principles](#principles)          
- [Features](#features)  
- [Installation](#installation)
    - [Option A: Using cargo (Windows/Linux)](#option-a-using-cargo-windowslinux)
    - [Option B: Using nix flakes (NixOS/Linux)](#option-b-using-nix-flakes-nixoslinux)
- [Configuration](#configuration)
- [Usage](#usage)
- [Development](#development)
- [Contributing](#contributing)

## Principles
- The agent should never write to the system without getting permission from the user.
- Guardrails should be implemented in code, not in prompts.
- The agent should act as an additional tool to the user not as a replacement for everything they already use.

## Features
- Minimal terminal UI
- Multi turn agent loop
- Agent memory
- Session and prompt history
- Full markdown rendering in terminal (thanks to [termimad](https://github.com/Canop/termimad))
- OpenAI Compatible API access
- Web search and extract using [Tavily](https://tavily.com)

## Upcoming Features
- Whatsapp and Discord gateways
- Agent skills
- MCP connectivity

## Tools
Currently Mia has the following tools:

|| Tool | Description | Permission Required |
|:-:|:-|:-|:-:|
| 📅 | `datetime`       | Get the current date and time              | No  |
| 📁 | `fs_list_dir`    | List files in a directory                  | No  |
| 📖 | `fs_read_file`   | Read a file from the filesystem            | No  |
| 🔍 | `fs_grep_files`  | Search file contents                       | No  |
| 🧭 | `fs_search_dirs` | Search for files in a directory            | No  |
| ✍️ | `fs_write_file`  | Write content to a file                    | Yes |
| 🧠 | `memory`         | Manage memory about the user and the agent | No  |
| 🐍 | `exec_python`    | Execute Python 3 code                      | Yes |
| 🐚 | `exec_shell`     | Run shell commands                         | Yes |
| 🪏 | `web_extract`    | Extract content from a URL                 | No  |
| 🌐 | `web_search`     | Search the web                             | No  |

## Installation
### Option A: Using cargo (Windows/Linux)
Mia requires rust nightly
```
rustup toolchain install nightly
rustup default nightly
```
Cargo can fetch, build and install mia.
```
cargo install --git https://github.com/mastermach50/mia-agent
```
### Option B: Using nix flakes (NixOS/Linux)
Run the given command to test out mia on your system.
```
nix run github:mastermach50/mia-agent
```
To install it, add to your system packages using the same flake.

## Configuration
Mia requires an [Openrouter API key](https://openrouter.ai/workspaces/default/keys) (can be obtained for free, [there are also free models](https://openrouter.ai/docs/guides/routing/model-variants/free)), currently this is the only way to access LLMs.
>Support for using any OpenAI compatible API is planned for the future (very soon).

1. On the first run, the agent will create all required folders and files, but will not start because of no API keys.
```
mia
```
2. Edit `~/.mia/.env` on Linux or `C:\Users\<username>\.mia\.env` on Windows and add
```
OPENROUTER_API_KEY=<your-openrouter-api-key>
```

3. Run `mia tui` to start the agent.

4. (Optional) Add your [Tavily API key](https://app.tavily.com/home) (can be obtained for free and has a reasonable number of free searches per month) to `.mia/.env` allow the `web_search` tool.
```
TAVILY_API_KEY=<your-tavily-api-key>
```

6. (Optional) Run `mia tools` to see if all the tools are usable and find which components are missing if any.

Currently the external tools required by mia are
- [Python](https://python.org)
- [Ripgrep](https://github.com/BurntSushi/ripgrep)
- [fd](https://github.com/sharkdp/fd)

## Usage
### Cli
Use `mia --help` to acces the full cli help menu.

### Tui
The tui supports certain commands use `/help` while in the tui to see them.

## Development
### MSRV
Mia uses rust nightly as it is required for `whatsapp-rust` and certain necessary features.

## Contributing
We DO NOT accept vibecoded contributions or contributions from agents.

However AI assisted code is fine as long as you the human are the one contributing.

Human contributions are always welcome.
