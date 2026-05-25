# n8n MCP for Zed

A Zed extension to access n8n directly in Zed using the Model Context Protocol (MCP).

## Configuration

Configure the extension under `context_servers.mcp-n8n.settings` in your Zed
settings. Set `server_url` to your instance-level n8n MCP endpoint, which looks
like `https://<your-n8n-host>/mcp-server/http` (see n8n's Settings →
Instance-level MCP page for the exact URL).

## Authentication

The `auth` setting selects how the extension authenticates against your n8n
instance. Two modes are supported:

### `oauth` (default)

```jsonc
{
  "server_url": "https://your-n8n-host/mcp-server/http",
  "auth": "oauth"
}
```

Uses [`mcp-remote`](https://www.npmjs.com/package/mcp-remote), which opens your
browser to log in to n8n on first run. Tokens are cached and refreshed
automatically in `~/.mcp-auth/`, so subsequent runs don't prompt again. Leave
`access_token` empty in this mode.

### `token`

```jsonc
{
  "server_url": "https://your-n8n-host/mcp-server/http",
  "auth": "token",
  "access_token": "<your-bearer-token>"
}
```

Uses [`supergateway`](https://www.npmjs.com/package/supergateway) to forward
requests over streamable HTTP with an `Authorization: Bearer <access_token>`
header. `access_token` is required in this mode — startup fails if it is empty.

## Resources

- [Zed Docs – Developing Extensions](https://zed.dev/docs/extensions/developing-extensions)
- [Zed Extensions - GitHub Repo](https://github.com/zed-industries/extensions)

## License

MIT
