# Axiv Pay MCP examples

Connect an AI agent to Axiv Pay, discover payment tools, and integrate
stablecoin checkout or hosted payment links from Rust.

- Product and account setup: <https://pay.axiv.cc/>
- Agent quickstart: <https://pay.axiv.cc/docs/agent-quickstart/>
- Official MCP Registry: [`cc.axiv/pay` 1.1.0](https://registry.modelcontextprotocol.io/v0.1/servers/cc.axiv%2Fpay/versions/1.1.0)
- Machine-readable contract: <https://pay.axiv.cc/openapi.json>

This repository contains client examples only. The payment service runs at
Axiv Pay; this is not a self-hostable copy of its backend.

## Choose your connection

| Endpoint | Authentication | Tools |
| --- | --- | --- |
| `https://pay.axiv.cc/public/mcp` | No token for discovery; your own Axiv Pay API token for payment tools | 12 tools, including quotes, payment creation, funding instructions, status and payment links |
| `https://pay.axiv.cc/mcp` | Per-client OAuth, scope `catalog:read` | Four read-only discovery tools; no payment privileges |

An OAuth discovery login does not issue a payment API key. Obtain your own
merchant API key through the wallet account flow linked from the product
site. Operator Cloudflare Access login is separate.

Use the public endpoint in a Streamable HTTP MCP client. For payment tools,
configure `Authorization: Bearer <your Axiv Pay API key>` in the client's
private credential configuration, never in tool arguments or committed JSON.
For an OAuth-capable agent that only needs discovery, connect `/mcp` instead.

## Run the Rust client

Install Rust, then:

```sh
git clone https://github.com/ontsbus/axiv-pay-examples.git
cd axiv-pay-examples
cargo run --locked -- list
cargo run --locked -- call search_capabilities requests/search.json
cargo run --locked -- call check_payment_route requests/route.json
```

These commands require no API key and move no funds. `list` returns the live
tool descriptions and input schemas. The route example checks Base USDC →
Arbitrum USDC for a recipient amount of 10. A quotable route is not proof of
a completed payout. Inspect `quotable`, `verifiedLive` and `liveRoutes` in
the response; do not advertise transfer verification that has not occurred.

## Create a payment link

Review `requests/payment-link.json`: it creates a single-use link for 10 USDC
on Arbitrum, paid to the account wallet belonging to your key. Change the
amount, destination, title and optional recipient for your actual order.
Then supply your token privately. For example, in Bash:

```bash
read -r -s -p 'Axiv Pay API token: ' AXIV_PAY_API_TOKEN
export AXIV_PAY_API_TOKEN
cargo run --locked -- call create_payment_link requests/payment-link.json --allow-write
unset AXIV_PAY_API_TOKEN
```

This creates a live link and prints its result. It does not fund the payment.
Calling `create_payment_link` again creates another link; do not blindly
retry it after a timeout. Share the returned URL with the payer. Confirm the
actual payment status and order terms before fulfillment.

## Integrate checkout

1. Call `create_quote`. Replace the placeholder wallet addresses in
   `requests/quote.json`; `amountAtomic` is a source-asset atomic amount.
2. Review chains, assets, recipient, payer, minimum receive, fees and expiry.
3. Call `create_payment` with the returned quote ID and a unique persisted
   idempotency key (`requests/payment.json` is a template). Keep the exact
   request. If its response is lost, reuse the same key and arguments.
4. Call `get_funding_step`. The payer's own wallet verifies and signs each
   returned transaction. This example never accepts a wallet private key,
   signs a transaction, or broadcasts one. Do not fund simulated responses.
5. After the wallet sends a transaction, call `submit_funding_transaction`
   with its payment ID, step ID and transaction hash. An unknown wallet
   result must be reconciled before signing again.
6. Poll `get_payment_status`. For the `EXTERNAL` settlement model, `PAID` is
   the completion state; `settlementStatus: NOT_TRACKED` refers to external
   solver reimbursement. Do not wait for `SETTLED` on that model. Match the
   order reference, destination wallet/chain/token and expected minimum
   amount before fulfilling. A submitted hash alone is not payment proof.

Use `cargo run --locked -- call TOOL arguments.json --allow-write` only after
reviewing the live schema. Read-only authenticated tools, including
`get_payment_status`, still require your own API token.

The client pins the production URL, refuses redirects, marks the token
header sensitive, and performs no automatic retries. Any tool not explicitly
marked read-only requires `--allow-write`. Preserve structured errors and
idempotency state when adapting the example into your application. The generic
client does not persist an order state machine for you.

## Validation

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

Tests cover write opt-in, authentication boundaries and refusal to forward
the caller's token through an HTTP redirect. CI uses only local fixtures;
it creates no live payment, link, account or wallet transaction.

Service policies: [Privacy](https://pay.axiv.cc/privacy/),
[Terms](https://pay.axiv.cc/terms/).
