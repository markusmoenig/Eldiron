# External player login

A persistent dedicated world can optionally broker OpenID Connect device login alongside username/password accounts. Native, WASM and foreign graphical Players use the same GPU screens and WSS messages. The provider performs browser authentication; the game never receives the provider password. Single player, Creator Play and accountless demos do not require an identity provider.

The supported profile uses discovery, the OAuth device authorization grant, `openid`, a request nonce and RS256-signed ID tokens. Select a provider that advertises `device_authorization_endpoint` and returns the nonce in its ID token. Providers offering only authorization-code redirects or plain OAuth access tokens need another adapter. Keycloak exposes a device grant and carries the request nonce in its device model; configure and validate your intended realm/client. See the [device grant specification](https://www.rfc-editor.org/rfc/rfc8628.html), [OpenID token-validation rules](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation), and [Keycloak's device endpoint](https://github.com/keycloak/keycloak/blob/main/services/src/main/java/org/keycloak/protocol/oidc/grants/device/endpoints/DeviceEndpoint.java).

## Private server configuration

Keep the configuration outside project files, releases and public web roots:

```toml
[[providers]]
id = "my_identity"
label = "My identity provider"
issuer = "https://identity.example/realms/game"
client_id = "eldiron-game"
# client_secret = "..."  # Optional confidential client; client_secret_post.
allowed_verification_hosts = []
```

Launch the existing saved server with `--identity-config private/identity.toml`. A public client needs no secret or provider configuration: Welcome advertises only provider IDs and labels. The issuer must exactly match discovery. Discovery and advertised device/token/JWKS endpoints require HTTPS and reject redirects. The verification link must be HTTPS and use the issuer's host or an explicitly allowed verification host. `allow_insecure_loopback = true` permits HTTP loopback solely for a local provider test.

A provider can be a public device client without a secret, or a confidential client configured for `client_secret_post`. Up to eight providers are configured per world. Startup rejects duplicate IDs, malformed configuration or failed discovery. Configure production credentials/TLS with your actual identity service; the repository test issuer is not an identity service for real users.

## Player workflow and ownership

Choose **Other login methods** in the lobby, select a provider and press Continue. Copy the verification link/code, authenticate in your browser and approve the code. The game waits without blocking its socket reader or world simulation. Cancel/Back stops polling. Device grants expire within ten minutes. Pending/slow-down responses honor provider polling intervals; terminal failures stop the flow.

To attach a provider to an existing password account, log in first, choose **Account → Link another login method**, enter the current game password and continue with provider authorization. A successful link retains the account UUID, characters, inventory and receipts. It commits the mapping and a security-revision increment, then signs out account sessions. Reconnect and choose the provider to sign in again.

The same screen can **Unlink provider**, using the current game password. Removal commits before token/socket revocation. The local credential remains available. A provider-only account must first establish a local credential through trusted operator recovery before linking another method or unlinking; this prevents losing its last usable credential. Provider-managed password/email recovery remains with that provider.

First-time provider login creates a world-scoped account with a stable issuer/subject mapping and no enabled local password. It does not automatically create a character. Passwordless records contain a hash of an independently random discarded secret, so older password readers cannot authenticate them with a shared placeholder. Existing records without the local-password flag default to password-enabled.

Identities never merge by email, display name or provider label. A provider identity already attached to another game account cannot be reassigned through the link screen. Unlinking removes its route to the old account; signing in afterward can create a fresh separate account. Account merging is not implicit. Each world's identity directory remains isolated even when it uses the same issuer as another world.

## Verification and revocation

The broker validates token signature/algorithm/key, exact issuer, audience, authorized party, expiration/not-before, issuance freshness and nonce. Keys come only from the configured issuer's discovered JWKS endpoint; token headers cannot supply a fetch URL. Key sets/responses/token sizes are bounded. Four broker flows and four password-hash jobs are allowed concurrently, under existing per-IP authentication limits.

No device code, client secret, ID/access token, issuer/subject directory or hash enters public exports or gameplay replication. Only the verification link and user code reach the initiating socket. Results are bound to its current attempt and captured account security revisions. Cancellation, disconnect, password changes, global logout and unlinking invalidate stale authorization results. Socket-owned tasks and HTTP requests are cancelled with their connection. A new credential/mapping is published only after durable commit.

[Account management](account-management.md) describes local password changes and private recovery; [the parity contract](network-gameplay-parity.md) describes the gameplay additions. Managed hosting, a creator identity/billing directory, SMTP delivery and automatic cloud deployment remain separate work.
