## v0.1.2
### Added
- `--free` to `mia model list` that is equivalent to `--max-price 0`.
- `mia models` is now an alias for `mia model`.

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