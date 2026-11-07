# Deploy the site

The site is static HTML, CSS, and a 404 page in `site/public`. There is no frontend build and no Worker script. Cloudflare Workers Static Assets serves that directory. This deploy does not build the CLI and does not start the local MCP server.

Config: `site/wrangler.jsonc`

- `assets.directory` is `./public`
- `assets.not_found_handling` is `404-page`
- `assets.html_handling` is `auto-trailing-slash`
- No `main` field and no `ASSETS` binding, because nothing runs at request time
- `routes` attach custom domains `tracelamp.dev` and `www.tracelamp.dev`
- `workers_dev` stays enabled. `preview_urls` stays off
- No account id is stored in the repo. Wrangler reads `CLOUDFLARE_ACCOUNT_ID` from the environment

`compatibility_date` is `2026-11-07`.

## Command

From a checkout, with Node available:

```sh
cd site
npm ci
npx wrangler deploy
```

`npm run deploy` is the same command. The uploaded directory is `site/public`. Wrangler does not compile it.

Do not run this until the GitHub secrets below exist if you are using Actions. A successful `wrangler deploy` creates the Worker and, because `custom_domain` is set, asks Cloudflare to create DNS records and a certificate for `tracelamp.dev` and `www.tracelamp.dev` in the zone that is already on Cloudflare. This repository does not name the target addresses. Leave the record values to that deploy. Do not point the domain at the MCP port.

## GitHub Actions

Workflow: `.github/workflows/deploy-site.yml`

It runs on `workflow_dispatch`, and on a push to `main` that changes `site/**` or the workflow file. Pull requests do not deploy.

Repository secrets, names only:

| Name | Use |
|---|---|
| `CLOUDFLARE_API_TOKEN` | Wrangler authentication. Do not commit the value. |
| `CLOUDFLARE_ACCOUNT_ID` | Account that owns the Worker. Do not commit the value. |

Create the token in the Cloudflare dashboard with the **Edit Cloudflare Workers** template. Scope it to the account that owns the `tracelamp.dev` zone. Attaching a custom domain also creates DNS records, so the token needs **Zone / DNS / Edit** on that zone if the Workers template does not already include it.

The action is `cloudflare/wrangler-action@v4` with `workingDirectory: site` and `command: deploy`.

## Workers Builds

Use either this GitHub Action or Workers Builds, not both. Both would deploy the same Worker on every push to `main`.

If you connect the GitHub repository from the Cloudflare dashboard instead:

| Setting | Value |
|---|---|
| Production branch | `main` |
| Root directory | `site` |
| Build command | empty. There is no build. |
| Deploy command | `npx wrangler deploy` |
| Non-production branch builds | off, until you want preview uploads |

Workers Builds can mint its own API token for that connection. You do not need to copy that token into the repo. If you use the GitHub Action above, leave Workers Builds disconnected.

The first successful production deploy is what attaches the custom domains. Certificate issuance happens after that, in Cloudflare, not in this repo.
