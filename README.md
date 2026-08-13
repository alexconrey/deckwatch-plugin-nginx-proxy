# deckwatch-plugin-nginx-proxy

A [deckwatch](https://github.com/alexconrey/deckwatch) plugin that provisions an nginx
reverse-proxy sidecar for an application via the **Infrastructure tab** — no annotations required.

## Usage

1. Open the Application detail page in deckwatch.
2. Click the **Infrastructure** tab.
3. Click **+ Add Nginx Proxy**.
4. Choose the image and port, then click **Provision**.

Once provisioned, deckwatch injects on the next reconcile:

- **Sidecar**: `nginx:1.27-alpine` on the selected port (50m CPU / 64Mi memory), named `nginx-proxy`
- **Annotation**: `deckwatch.io/nginx-proxy: "provisioned"` on all deployments

No application-level env vars are injected (nginx manages its own configuration).

To remove the sidecar, click **Deprovision** on the Infrastructure tab. The sidecar is removed
from all deployments on the next reconcile via server-side apply.

## Configuration

### Resource fields (per-provisioned instance)

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `NGINX_IMAGE` | String | `nginx:1.27-alpine` | nginx container image |
| `NGINX_PORT` | Select | `80` | Port the nginx sidecar listens on (`80`, `443`, `8080`) |

### Plugin config (global defaults)

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `NGINX_IMAGE` | String | `nginx:1.27-alpine` | Default nginx image (overridable per-resource) |

Set global defaults in **Settings → Plugins → Nginx Proxy → Configure**.

## Building

```bash
# Install the wasm32 target (once)
rustup target add wasm32-unknown-unknown

# Build the WASM artifact
make build          # → dist/plugin.wasm

# Run host-target unit tests
make test
```

## Marketplace

This plugin is available on the [deckwatch marketplace](http://market.deckwatch.io).

To install via the deckwatch UI: **Settings → Plugins → Browse Marketplace → Nginx Proxy**.

### Submitting to the marketplace

To add or update this plugin in the marketplace catalog, open a PR against
[alexconrey/deckwatch-marketplace](https://github.com/alexconrey/deckwatch-marketplace)
adding the following entry to `catalog.json`:

```json
{
  "name": "Nginx Proxy",
  "slug": "deckwatch-plugin-nginx-proxy",
  "description": "Provisions an nginx reverse-proxy sidecar via the Application Infrastructure tab.",
  "author": "alexconrey",
  "homepage": "https://github.com/alexconrey/deckwatch-plugin-nginx-proxy",
  "trust_level": "community",
  "tags": ["nginx", "proxy", "sidecar"],
  "latest_version": "0.1.0",
  "source": {
    "type": "github",
    "repo": "alexconrey/deckwatch-plugin-nginx-proxy",
    "ref": "v0.1.0",
    "path": "plugin.wasm",
    "use_release": true
  },
  "allowed_hosts_hint": []
}
```

Also add `plugins/deckwatch-plugin-nginx-proxy.md` following the template in the marketplace repo.

## License

Apache-2.0
