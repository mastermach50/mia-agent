# Short Term
<<<<<<< New base: Added working_dir option to exec_python and exec_shell
<!--## TUI
||||||| Common ancestor
## Tools
- Offset in fs_read_file and fs_write_file
- Timeout in exec_shell
- New fs_edit_file tool
- Max results and include hidden options in fs search type tools

## TUI
=======
## Tools
- Offset in fs_read_file and fs_write_file
- Timeout in exec_shell and exec_python
- New fs_edit_file tool
- Max results and include hidden options in fs search type tools
- Stateful shell

## TUI
>>>>>>> Current commit: Added working_dir option to exec_python and exec_shell
- Implement message history for input box
- Improve tui permission asking ui
- Tool call status in tui-->

## Code Quality
- Provider registry similar to tool registry
- setup.rs is very fragile, needs to be reimplemented after provider registry

## Features
- Optional Local/Custom endpoint api key
- Cost estimation
- Re implement single prompt command
- Implement session title generation
- Make session pruning
- Context compression
- Ultra light mode

# Long Term
## Features
- Diffed config edits
- Graceful error handling in API calls
- Setup custom colors
- Add skills support
- Git style diffs
- Add MCP support
