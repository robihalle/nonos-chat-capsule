# Install NONOS Chat

NONOS Chat is a native x86_64 NONOS capsule. The download is a signed `.nonos` package, not a Linux program or an operating-system image.

## Compatibility first

This preview is signed by the private publisher identity used for the NONOS Chat development image. The receiving NONOS kernel must trust its certificate chain and STARK attestation root. Matching the NONOS version alone is not enough.

The release metadata records the pinned upstream revision and SHA-256 fingerprints of the trust policy and attestation root. A stock official NONOS image is not currently an approved installation target. The project does not have an official NONOS publisher certificate.

Do not disable verification or replace your system trust root just to install this package. A package rejected with an access error needs a compatible, explicitly trusted publisher release.

The receiving system needs the installer, VFS package store, terminal/desktop package UI, network, TLS, toolkit, window manager and compositor services. The install name `chat` and service `app.chat` must be available.

## Download and verify

Download `nonos-chat-0.1.0.nonos`, `release.json` and `SHA256SUMS` from the same authenticated release location. Repository access is required while the project is private.

On your host computer, compare the package SHA-256 with `release.json`. With the pinned upstream host tools you can additionally run:

```sh
nonos-pack verify --in nonos-chat-0.1.0.nonos
```

This checks the package's Ed25519 and ML-DSA-65 signatures. It does not establish that the publisher is trusted by your particular NONOS kernel; the native installer performs that admission check.

Transfer the package into the guest VFS at `/pkgs/nonos-chat-0.1.0.nonos`. A file downloaded in the host browser is not automatically inside the VM. Use a supported VFS transfer path or the upstream package-store disk tool. The repository documents the tested transfer method in `docs/PACKAGING.md`.

## Install in NONOS

Open Terminal and run:

```text
nox pkg install /pkgs/nonos-chat-0.1.0.nonos
```

Review the verified publisher namespace and permissions. Chat requests CoreExec, Network, IPC, Memory, Crypto, GraphicsDisplayQuery and GraphicsSurfaceCreate. It does not request Admin, Hardware, FileSystem or StoreWrite.

Then confirm installation:

```text
nox pkg install /pkgs/nonos-chat-0.1.0.nonos --yes
```

Open Launchpad and select **chat** in the installed applications section. This asks the installer to load the verified capsule and sends the focus event that opens its window. The terminal command `nox install chat` loads the service, but the graphical app may still wait for that focus event.

The desktop also discovers `.nonos` files in `/pkgs` and provides an installation consent dialog. GUI and persistence validation status is recorded in `docs/PACKAGING.md`.

## Connect your own provider

For Groq Free Plan:

- API URL: `https://api.groq.com/openai/v1`
- Model: `openai/gpt-oss-20b`
- API key: your own Groq key

For OpenRouter free models:

- API URL: `https://openrouter.ai/api/v1`
- Model: `openrouter/free`
- API key: your own OpenRouter key

No API key is included. Keys and chat history exist only in the running capsule session. Free-provider quotas still apply. When using a remote keyboard, ensure colons, underscores and uppercase key characters are entered correctly.

## Remove or update

Close Chat, then run:

```text
nox pkg remove chat
```

Version 0.1.0 uses the upstream installer's remove-then-install update flow; it is not an atomic upgrade mechanism. A duplicate install is rejected. On a VM using a disposable disk snapshot, package-store changes do not survive discarding that snapshot.
