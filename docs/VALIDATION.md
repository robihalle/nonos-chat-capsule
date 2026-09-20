# Deployment validation

Validated on September 20, 2026 with the pinned upstream revision in `UPSTREAM_REV`.

## Build and native runtime

- 21 host tests pass locally and in GitHub Actions: 10 protocol, 4 VirtIO geometry, 3 TLS framing and 4 transport-state tests.
- [CI run for the deployed application source](https://github.com/robihalle/nonos-chat-capsule/actions/runs/35536345139) passed.
- All 47 selected capsules are signed and enrolled. Image receipt gates for manifests, declared capabilities, ledger, root embedding and STARK admission pass.
- The image boots in isolated QEMU, and the native Chat launcher opens the English application.
- Image SHA-256: `efc9980fef01f9d9760b81f81b028af89a74f52648dd0ee5499c6286f06e2ca95`.
- The original image and a deployment rollback snapshot are retained on the host.
- The VM has no swap allowance. API keys and conversations remain session-only.

## OpenRouter integration

Configured API base: `https://openrouter.ai/api/v1`.
Default model: `openrouter/free`.
A dedicated API key was entered into the running guest; it is absent from the repository and image.

Native guest requests establish TCP, validate the TLS server flight, send the authenticated request and parse an HTTP 429 response. Both the free router and an explicit free model reached the API. No certificate, manifest, signature or admission checks were disabled.

The same dedicated key was tested through OpenRouter's official API playground with an explicit `openrouter/free` user message. The API returned:

- Error: `Rate limit exceeded: free-models-per-day-high-balance.`
- Limit source: `openrouter_free_tier_daily`.
- Daily limit: 1000; remaining: 0.
- Reset reported by the API: September 21, 2026 at 00:00 UTC.

This is an account-level free-model quota blocker. Creating another key or changing between free models does not establish a fresh account quota. The key's spending limit remains USD 0; no paid model or credit purchase was used.

**A successful generated model answer is still unverified.** After the quota resets, send a short message in the native app to complete that validation. The current UI reports HTTP 429 as “Provider limit reached. Please try again later.”

## Compatibility fixes exercised

- Respect QEMU's actual 1024-entry legacy VirtIO queues with aligned ring geometry and bounded posted buffers.
- Treat an empty/transient socket receive as pending until a bounded deadline.
- Accept a complete encrypted TLS record containing a coalesced server flight instead of requiring three records. Upstream certificate and Finished verification still gate application-data transmission.

## Operational notes

Use the full versioned API base URL. Enter the model manually for OpenRouter: its provider-wide model catalog may exceed the application's 256 KiB response bound.

Closing the capsule or restarting the VM clears the API key and chat history. Re-enter the key for a new session. Use New chat to clear the conversation or Clear key to erase the credential while the app remains open.
