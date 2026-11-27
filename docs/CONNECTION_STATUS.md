# Axiv Pay connection status

Updated November 27, 2026, Asia/Seoul.

## Recommended addresses

- Payment/discovery MCP: `https://pay.axiv.cc/public/mcp`.
- OAuth discovery only: `https://pay.axiv.cc/mcp`, `catalog:read`, four read-only tools.
- Optional direct payment/discovery MCP: `https://api.pay.axiv.cc/site/public/mcp`.

The canonical payment address is the default in this Rust client and in the
[official Registry record](https://registry.modelcontextprotocol.io/v0.1/servers/cc.axiv%2Fpay/versions/1.1.0).
It reaches the existing Oracle service through Cloudflare Tunnel. The direct
alternative uses the same caller API-key permissions; selecting it does not
replace the OAuth issuer or grant payment permissions. The client never
automatically fails over or retries writes.

This repository contains public **client examples**. Its URL must not be
presented as the source repository for the hosted server's private backend.

## Evidence and limits

The publisher's post-cutover rollout recorded these first-attempt reads on
November 27, 2026, 00:54:48–00:57:09 UTC:

| Path | Passed | Observation |
| --- | --- | --- |
| Canonical MCP | 100/100 | Tunnel response marker present |
| Canonical site health | 100/100 | Tunnel response marker present |
| Direct Oracle MCP control | 100/100 | Existing direct address |

This table is the publisher's bounded rollout record, not an independent
audit, settlement test or future availability guarantee. It does not prove
the old inbound network path was permanently repaired. The prior November 26
timeout investigation predates the current canonical transport and is not
the current recommendation for choosing a connection.

For this documentation update, the published Rust client was checked again
against both payment-MCP addresses with `list` (12 tools each), and against the
canonical address with a successful `search_capabilities` call. The health
response was `ok`; canonical MCP/health responses carried the Tunnel marker.
The [dated check record](connection-check-20261127.json) separates these fresh
observations from the historical 100-sample rollout. These read-only checks
used no payment token, created no payment or link, and signed/sent no wallet
transaction.

Independent records have their own dates and scope:

- [Glama connector](https://glama.ai/mcp/connectors/cc.axiv/pay): discovery listing and connection entry point.
- [ProtocolProbe](https://protocolprobe.com/mcp/servers/cc.axiv/pay): reports November 26 initialize/inventory inspection; it states that no tool was called.
- [Lowdown](https://lowdown-proxy.vercel.app/servers/cc.axiv/pay): reports November 27 initialize/tools-list observation; it states no agent-reported interactions.
- [Agent Data Census](https://agents.datacensus.org/mcp/new/): Registry-derived new-server listing.

A directory record, a tool inventory or a successful search is not proof that
all payment tools were exercised, that a wallet paid, or that an independent
editor recommends the service.

## Repeat the client checks

From the repository root:

```bash
cargo run --locked -- list
cargo run --locked -- call search_capabilities requests/search.json
AXIV_PAY_MCP_URL=https://api.pay.axiv.cc/site/public/mcp cargo run --locked -- list
```

Expect 12 payment/discovery tool definitions at each payment address. Catalog
search works without a token; payment and account tools still require the
caller's own Axiv Pay API key. Inspect `quotable`, `verifiedLive` and
`liveRoutes` separately before making claims about a route's payment history.
