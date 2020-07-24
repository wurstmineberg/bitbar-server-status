This is a simple [BitBar](https://getbitbar.com/) plugin that uses [our API](https://wurstmineberg.de/api/v3) to show who is currently online on the main world.

# Installation

1. [Install BitBar](https://getbitbar.com/).
    * If you have [Homebrew](https://brew.sh/), you can also install with `brew cask install bitbar`.
2. [Install Rust](https://www.rust-lang.org/tools/install).
    * If you have Homebrew, you can also install with `brew install rust`.
3. Install the plugin:
    ```sh
    cargo install --git=https://github.com/wurstmineberg/bitbar-server-status
    ```
4. Create a symlink to `~/.cargo/bin/bitbar-wurstmineberg-status` in your BitBar plugin folder. Name it something like `wurstmineberg.45s.o`, where `45s` is the rate of update checks.
5. Refresh BitBar by opening a menu and pressing <kbd>⌘</kbd><kbd>R</kbd>.
