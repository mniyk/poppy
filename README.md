# Poppy

A keyboard-driven launcher for Windows. Press a hotkey and it pops up: launch apps, open bookmarks, search the web. Named after that "pop". Built with Rust and Dioxus.

## Features

- **Global hotkey** — summon it from anywhere with `Ctrl+Alt+R`
- **App launcher** — search and start anything in your Start Menu
- **Bookmarks** — open URLs you've registered in a config file
- **Web search** — search with Google or DuckDuckGo
- **Runs in the tray** — stays resident after you close it, so the next call is instant

## Usage

| Action | Key |
| --- | --- |
| Show / hide | `Ctrl+Alt+R` |
| Move through results | `↑` `↓` |
| Run | `Enter` |
| Close | `Esc` |

Clicking outside the window also closes it.
To exit, right-click the tray icon and choose **Quit**.

## Configuration

Bookmarks live in a TOML file.

    %APPDATA%\poppy\config\bookmarks.toml

A starter file is created on first launch. You can also open it from Poppy itself:
type `config` and run **Open Bookmarks**.

    [[bookmark]]
    name = "GitHub"
    url = "https://github.com"
    keywords = ["gh"]

`keywords` are aliases that will match in search. Edits take effect the next time you summon Poppy.

## Installation

Download the installer from [Releases](https://github.com/mniyk/poppy/releases) and run it.

## Development

    dx serve                                # start the dev server
    dx bundle --release --platform desktop  # build the installer

Built with [Dioxus](https://dioxuslabs.com/) 0.7.
Tailwind CSS is compiled by the Dioxus CLI, so no extra setup is needed.

## License

MIT
