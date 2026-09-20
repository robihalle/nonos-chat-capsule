# NONOS Chat Capsule

Native Rust chat application for NONOS, with a configurable OpenAI-compatible HTTPS API.
This repository is an application overlay for a pinned NONOS microkernel checkout, not a Linux web application.

**Status:** native capsule compiles (1.4 MiB ELF) and passes the declared-capability check; all 10 protocol tests pass locally and in GitHub Actions. Complete image build and VM integration are in progress.
Do not interpret a successful host test as evidence of a working guest network or a completed model request.

## First version
- Native NONOS window, configuration fields and scrolling conversation.
- HTTPS API base URL, masked API key and explicit model ID.
- Test connection / list models via GET `<base>/models`.
- Text chat via POST `<base>/chat/completions`, `stream: false`.
- Cancellation, bounded request/response sizes and explicit errors.
- The API key and conversation are session-only. No key is embedded in the image or written to a file.

Enter a complete versioned base URL such as `https://api.openai.com/v1`.
A different compatible provider may require a different prefix and model ID.
Use **Verbindung testen** to list models, then enter one in **Modell**.
A provider without a models endpoint can still be used by entering its model ID manually.
Use Tab to switch fields, Ctrl+A to replace a field, Enter to send, Shift+Enter for a newline.
Ctrl+V uses the NONOS clipboard, which is separate from the browser clipboard.

## Build on Linux
Dependencies: git, Rustup, C/C++ toolchain, clang/lld, pkg-config, libssl-dev,
QEMU, OVMF, swtpm, mtools, gdisk, xorriso, nasm and Python 3.
Use a dedicated non-root build account. On a small VPS use one Cargo job and provision swap.

```sh
git clone https://github.com/robihalle/nonos-chat-capsule.git
cd nonos-chat-capsule
git clone https://github.com/NON-OS/microkernel.git build/microkernel
git -C build/microkernel checkout "$(cat UPSTREAM_REV)"
git -C build/microkernel submodule update --init --recursive
rustup toolchain install nightly-2026-01-16 --profile minimal --component rust-src,llvm-tools-preview,clippy,rustfmt
scripts/build.sh "$PWD/build/microkernel"
```

The integration script adds a separate `app.chat` capsule and a **Chat** launcher entry.
The image uses `microkernel-desktop-base` with the STARK admission gate: graphical applications,
browser, terminal and network services remain present; optional std CLI tools are omitted.
The final raw QEMU/USB image is `build/microkernel/target/nonos.img`. Networking uses the upstream socket IPC and TLS implementation.
The build initializes a private image signing identity and enrolls the resulting capsule measurements.
It does **not** disable manifest checks, TLS verification or STARK admission.
Keep the upstream checkout's `.keys/` directory private. These keys identify this custom image,
not an official NONOS release.

## Tests
```sh
cargo +nightly-2026-01-16 test --manifest-path tests/Cargo.toml
```
Tests cover UTF-8 and JSON serialization, custom base paths, request injection,
fragmented fixed-length and chunked HTTP, size limits, API errors and model lists.

## Current limits
- No streaming, tool execution, attachments, persistent key vault or saved chat history.
- HTTPS only; DNS names and IPv4 literals can be parsed, but certificate support depends on upstream TLS.
- API key uses Bearer authentication. Provider-specific auth schemes are not implemented.
- A single request including conversation is limited to 14 KiB; start a new chat when it fills.
- Up to 256 KiB of response data; 180-second total request deadline.
- Text entry follows the current NONOS keyboard translation; clipboard paste supports UTF-8.
- The GUI network state machine is cancellable between polls; upstream DNS/TCP IPC can block briefly.
- NONOS itself is prerelease software. Guest network and TLS interoperability require VM verification.

## Layout
- `capsule/`: native application, protocol and transport.
- `scripts/`: pinned upstream integration and private image build.
- `tests/`: host tests for the exact application protocol source.
- `UPSTREAM_REV`: exact microkernel revision.

## License and upstream
AGPL-3.0-or-later. Integration reuses NONOS socket adapters, keymap and kernel capsule mirror
with their existing copyright notices.
Upstream: https://github.com/NON-OS/microkernel
