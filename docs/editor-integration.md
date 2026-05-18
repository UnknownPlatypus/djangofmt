# Editor integration

`djangofmt` reads from `stdin` when invoked with `--stdin-filename <PATH>`. The
profile (`django` / `jinja`) is inferred from the file extension.

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
