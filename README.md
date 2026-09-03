# Poppy

A keyboard-driven launcher for Windows. Press a hotkey and it pops up: launch apps, open bookmarks, search the web. Named after that "pop". Built with Rust and Dioxus.

## Features

- **Global hotkey** — summon it from anywhere with `Ctrl+Alt+R`
- **Window switcher** — jump to any open window by name
- **App launcher** — search and start anything in your Start Menu
- **Bookmarks** — open URLs you've registered in a config file
- **Snippets** — copy registered text (signatures, commands, etc.) to the clipboard
- **Projects** — open a project folder in VS Code
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

Config files live here:

    %APPDATA%\poppy\config\

Starter files are created on first launch. You can also open them from Poppy itself —
type `open` and run **Open Bookmarks**, **Open Snippets**, or **Open Projects**.

Edits take effect the next time you summon Poppy.

### bookmarks.toml

    [[bookmark]]
    name = "GitHub"
    url = "https://github.com"
    keywords = ["gh"]

### snippets.toml

    [[snippet]]
    name = "メール署名"
    content = "Yohei Kono"
    keywords = ["sig", "signature"]

### projects.toml

    [[project]]
    name = "poppy"
    path = "C:\\Users\\mniyk\\Work\\poppy"
    keywords = ["launcher"]

`keywords` are aliases that will match in search.

Opening a project requires the `code` command to be on your PATH
(enable it when installing VS Code).

## Installation

Download the installer from [Releases](https://github.com/mniyk/poppy/releases) and run it.

## Development

    dx serve                                # start the dev server
    dx bundle --release --platform desktop  # build the installer

Built with [Dioxus](https://dioxuslabs.com/) 0.7.
Tailwind CSS is compiled by the Dioxus CLI, so no extra setup is needed.

## License

MIT
