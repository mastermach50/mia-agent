## v0.2.0
### Added
- __New ratatui based tui is the default tui__.
    - The old tui can still be accessed using a hidden command `mia old-tui`
- new `/yolo` command in tui to always respond yes to permission requests.
- New `doc_convert` tool that allows the agent access to pandoc.

## v0.1.5
### Added
- More LLM providers
    - Google AI Studio
    - Groq
    - Cerebras
- Updated system prompts to make the agent more intelligent.
- New `mia session clear` command.
- Added experimental ratatui ui (`mia ratatui`).

### Changed
- Changed, session structure, older sessions will be invalid.

## v0.1.4
### Added
- Added experimental streaming support.
    - new `streaming` option in config.

### Changed
- Changed, session structure, older sessions will be invalid.

## v0.1.3
### Added
- `mia setup` command to quickly setup the agent.

### Fixed
- tui `/model` command output formatting.

### Removed
- Removed the `document` section from `config.toml` and moved it to the internal config. If it is present in an existing config then that section is ignored by the parser.

## v0.1.2
### Added
- `--free` to `mia model list` that is equivalent to `--max-price 0`.
- `mia models` is now an alias for `mia model`.
- Non overwriting sessions.
- `mia sessions list` to view all sessions.

### Fixed
- `--max-price` not evaluating properly

## v0.1.1
### Added
- Multiline inputs (Shift+Enter/Alt+Enter) and file pasting in tui.
- GitHub action for release builds.

### Fixed
- Race condition in Ctrl-C handler in `agent_loop::run_agent`.
- Fix changes requested by clippy.

### Removed
- Gateway code (for now, will be made soon).

## v0.1.0 Initial Release
The first release of Mia Agent. Currently it has:

- Working TUI
- OpenAI API Compatibility
- Agent Tools
- And many more...