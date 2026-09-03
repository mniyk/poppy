# Poppy

A keyboard-driven launcher for Windows. Press a hotkey and it pops up: launch apps, open bookmarks, search the web. Named after that "pop". Built with Rust and Dioxus.

## Features

- **Global hotkey** — summon it from anywhere with `Ctrl+Alt+R`
- **Window switcher** — jump to any open window by name
- **Clipboard history** — search and re-copy anything you've copied recently (last 30 items)
- **App launcher** — search and start anything in your Start Menu
- **Bookmarks** — open URLs you've registered in a config file
- **Snippets** — copy registered text (signatures, commands, etc.) to the clipboard
- **Projects** — open a project folder in VS Code
- **Web search** — search with Google or DuckDuckGo
- **Ask AI** — send your query to a local [Ollama](https://ollama.com/) model and read the answer inline
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

`content`を複数行にしたい場合は、TOMLのトリプルクォート(`"""..."""`)を使ってください。
普通の`"..."`の中には実際の改行を直接書けません。

    [[snippet]]
    name = "print関数"
    content = """
    def print():
        print("test")
    """
    keywords = ["print"]

### projects.toml

    [[project]]
    name = "poppy"
    path = "C:\\Users\\mniyk\\Work\\poppy"
    keywords = ["launcher"]

`keywords` are aliases that will match in search.

Opening a project requires the `code` command to be on your PATH
(enable it when installing VS Code).

### Ask AI

Type anything and the top result offers to ask AI about it. Press `Enter` on it to send
the query to [Ollama](https://ollama.com/) and see the answer, `Esc` to go back to search.

Requires Ollama running locally (or reachable over the network) with a model already
pulled (`ollama pull gemma3:4b`, for example). Set the host and model name in
**Settings** (`Ctrl+,`).

## Installation

Download the installer from [Releases](https://github.com/mniyk/poppy/releases) and run it.

## Development

    dx serve                                # start the dev server
    dx bundle --release --platform desktop  # build the installer

Built with [Dioxus](https://dioxuslabs.com/) 0.7.
Tailwind CSS is compiled by the Dioxus CLI, so no extra setup is needed.

## License

MIT
