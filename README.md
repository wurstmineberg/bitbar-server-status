This is a simple [BitBar](https://getbitbar.com/) plugin that uses [our API](https://wurstmineberg.de/api/v3) to show who is currently online on the main world.

# Installation

1. [Install BitBar](https://getbitbar.com/).
2. [Install Rust](https://www.rust-lang.org/tools/install).
3. Install the plugin:
    ```sh
    cargo install --git=https://github.com/wurstmineberg/bitbar-wurstmineberg-status
    ```
4. Create a symlink to `~/.cargo/bin/bitbar-server-status` in your BitBar plugin folder. Name it something like `wurstmineberg.45s.o`.
5. Refresh BitBar by opening a menu and pressing <key>⌘</key><key>R</key>.
