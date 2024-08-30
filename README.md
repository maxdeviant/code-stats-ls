# code-stats-ls

A language server for [Code::Stats](https://codestats.net/).

## Installation

See the [releases](https://github.com/maxdeviant/code-stats-ls/releases) page for pre-built binaries.

If you would like to install the Code::Stats language server from source you can run:

```sh
git clone git@github.com:maxdeviant/code-stats-ls.git
cd code-stats-ls
cargo install --path .
```

## Authentication

In order to authenticate with the Code::Stats API, the language server needs to know your API token.

You can generate or retrieve your API token from the [Machines](https://codestats.net/my/machines) page.

## Configuration

You can provide configuration to the language server in two ways:

- Via environment variables
- Via the configuration file (`~/.config/code-stats/config.toml`)

The following values are configurable:

| Name      | `config.toml` key | Environment variable   | Default value           |
| --------- | ----------------- | ---------------------- | ----------------------- |
| API Token | `api_token`       | `CODE_STATS_API_TOKEN` | None                    |
| API URL   | `api_url`         | `CODE_STATS_API_URL`   | `https://codestats.net` |
