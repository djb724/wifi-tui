# WIFI-tui

Connect to wireless access point through NetworkManager.

This project is primarily built just for myself and for the challenge of building a functioning system with minimal dependencies. Might upload to crates or AUR at some point in the future.

## What is this?

Network Manager is one of two primary internet connection daemons that you commonly see on linux systems (the other being IWD). 

Network Manager itself generally comes bundled with CLI and TUI integrations to allow users to interface with the system but I find both to be clunky and not very ergonomic to use, so I made my own. The visual design and layout takes influence from services like vim and netrw making it familiar to users of those.

### TODO items:
- Use differential buffers to prevent flickering on update
- flesh out cl arguments like --version and --help
