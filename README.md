This is a [BitBar](https://getbitbar.com/) plugin that shows who is currently online on [Wurstmineberg](https://wurstmineberg.de/).

For an equivalent Windows app, see [wurstmineberg/systray](https://github.com/wurstmineberg/systray).

# Installation

1. [Install BitBar](https://getbitbar.com/).
    * If you have [Homebrew](https://brew.sh/), you can also install with `brew cask install bitbar`.
2. [Install Rust](https://www.rust-lang.org/tools/install).
    * If you have Homebrew, you can also install with `brew install rust`.
3. Install the plugin:
    ```sh
    cargo install --git=https://github.com/wurstmineberg/bitbar-server-status --branch=main
    ```
4. Create a symlink to `~/.cargo/bin/bitbar-wurstmineberg-status` in your BitBar plugin folder. Name it something like `wurstmineberg.45s.o`, where `45s` is the rate of update checks.
5. Refresh BitBar by opening a menu and pressing <kbd>⌘</kbd><kbd>R</kbd>.

# Configuration

You can optionally configure the behavior of the plugin by creating a [JSON](https://json.org/) file at `bitbar/plugins/wurstmineberg.json` inside an [XDG](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html) config directory. All entries are optional:

* `showIfEmpty`: If `false`, the plugin is hidden entirely if the server is running but no players are online. Defaults to `false`.
* `showIfOffline`: If `false`, the plugin is hidden entirely if the server is not running. Defaults to `false`.
* `singleColor`: If `true` and exactly one player is online, the plugin's icon and the “1” player count text are colored in that player's favorite color, as set in their Wurstmineberg preferences. Defaults to `true`.
* `versionLink`: One of the following:
    * `true`: Clicking on the version info menu item opens the [Minecraft Wiki](https://minecraft.fandom.com/) article for that version. This is the default.
    * `"alt"`: Holding <kbd>⌥</kbd> turns the version info menu item into a link to the Minecraft Wiki article for that version.
    * `false`: The version info item is still displayed but cannot be clicked.
* `zoom`: A number indicating the logical pixel scale, e.g. `2` on most modern Mac displays. Defaults to `1`.
