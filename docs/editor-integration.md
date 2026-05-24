# Editor integration

`djangofmt` reads from `stdin` when invoked with `--stdin-filename <PATH>`.
The profile (`django` / `jinja`) is inferred from the file extension.

## Helix

`~/.config/helix/languages.toml`:

```toml
[[language]]
name = "html"
formatter = { command = "djangofmt", args = ["--stdin-filename", "%{buffer_name}"] }
auto-format = true

[[language]]
name = "jinja"
scope = "source.jinja"
file-types = ["jinja", "jinja2", "j2"]
roots = []
formatter = { command = "djangofmt", args = ["--stdin-filename", "%{buffer_name}"] }
auto-format = true
```

## PyCharm

### Via File Watcher

`djangofmt` can be installed as a File Watcher in PyCharm to run on every save.
Open the Settings pane, then navigate to `Tools`, then `File Watchers`.
From there, add a new watcher with one of the following configurations.

#### Via `pre-commit` (recommended)

Running `djangofmt` through `pre-commit` ensures the version and configuration used in the editor match the one
enforced by your project's hooks.

![Install djangofmt as a File Watcher via pre-commit](https://github.com/user-attachments/assets/TODO)

#### Via `djangofmt` directly

Alternatively, invoke the `djangofmt` binary directly.

![Install djangofmt as a File Watcher directly](https://github.com/user-attachments/assets/TODO)

## Zed

`~/.config/zed/settings.json`:

```json
{
  "languages": {
    "HTML": {
      "formatter": {
        "external": {
          "command": "djangofmt",
          "arguments": ["--stdin-filename", "{buffer_path}"]
        }
      },
      "format_on_save": "on"
    },
    "Jinja2": {
      "formatter": {
        "external": {
          "command": "djangofmt",
          "arguments": ["--stdin-filename", "{buffer_path}"]
        }
      },
      "format_on_save": "on"
    }
  }
}
```
