# Package distribution

## Release scope

Version 0.1.0 is a private preview in the upstream NOS1 / `.nonos` format. It contains the actual Chat ELF, signed manifest, publisher certificate and STARK trailer. The outer container is signed with Ed25519 and ML-DSA-65.

It requires a host that already trusts this certificate chain and attestation root. End-to-end installation is not yet validated. It is not currently certified for stock official NONOS images. The native installer is the authority; the release does not change trust policies or bypass verification.

## Build the package

First build the signed application/image using `scripts/build.sh`, then build the upstream packaging tool:

```sh
cargo +nightly-2026-01-16 build --release --locked --manifest-path build/microkernel/tools/nonos-pack/Cargo.toml
python3 scripts/package.py build/microkernel --out dist/0.1.0
python3 scripts/test-package.py build/microkernel dist/0.1.0
```

The output directory must be new. Only a fixed allowlist is published: the package, release metadata, checksums, installation guide and license. No signing seed, API key, VM state or chat data is copied. Keep source access available to every binary recipient under AGPL-3.0-or-later.

The metadata records exact artifact hashes and the host trust fingerprints. A SHA-256 match proves byte identity, not publisher authorization.

## Transfer from a host browser into a VM

A download in Chrome is on the host computer, outside the guest VFS. The current pinned terminal's `nox pull` transport is plain HTTP; the package workflow does not send private download credentials over that transport.

For an offline transfer using only Python 3:

```sh
python3 scripts/make-package-disk.py --package nonos-chat-0.1.0.nonos --out chat-packages.img
```

This creates a new 128 MiB sparse disk with the upstream package store at LBA 256. Existing files/disks cannot be overwritten. Attach it as the guest's first VirtIO block data disk; use separate SATA/IDE boot media. The VFS exposes the package under `/pkgs`. Keep the disk writable if installations should persist.

This is a VM transfer method, not a universal USB auto-mount implementation. A bare-metal or differently configured host needs its own supported VFS transfer path.

## Clean installation test host

`scripts/build-package-host.py <upstream>` builds a separate signed desktop profile from the existing enrollment with automatic Chat spawn disabled. It preserves all production certificate and STARK gates. It never deploys or restarts the production VM.

This matters because the normal development image already starts `app.chat` at boot: installing over that active built-in service is correctly rejected. Merely repackaging the app and clicking its built-in launcher would not test external package loading.

The package host shares the development trust policy, so this test establishes compatibility with that trust domain, not arbitrary official NONOS installations.

## Validation

Host checks passed: package checksum, both publisher signatures, exact four-section roundtrip, certificate/manifest/ELF binding, rejection of eight damaged packages and a stale ELF. The official upstream trust policy rejects our private publisher certificate. Four Python tests cover the transfer disk layout, exact payload, overwrite protection and invalid/oversized inputs.

Native validation is blocked, not passed. On the isolated 2 GiB package-host VM with automatic Chat spawn disabled:

- A candidate package (SHA-256 `8e7b933191de287086c03a13f018ca1f5914acc457920365306f51552f355830`) passed the native admission query and displayed the expected private publisher namespace and capabilities.
- Confirmed installation returned `pkg: store write failed`; the kernel logged `[STORE-WR] err`. No successful persistent installation or external application launch was established.
- A kernel rebuilt with a more detailed store-error log subsequently returned `pkg: malformed package or path` before reaching the write path. The file remained visible with the expected 1,777,308-byte size. This reproduced with one or two staged files and with the earlier transfer disk, so a multiple-file explanation was not established.
- The transfer helper now reserves the full 64-entry TOC region. A regression test verifies that rewriting that whole region cannot overwrite the downloaded package.
- Native damaged-package rejection, duplicate-install refusal, removal, external launch and persistence after reboot remain unverified.

The authenticated download is the original packaged preview (SHA-256 `3a375e2760d6a92da418b26b6dbea2e0fbc410e0d1269a5b742cbde3628a36e8`), not the later test-host candidate. Both passed host-side package validation. Neither is presented as a completed native installation.

Production remained on its existing working Chat image; diagnostic builds were not deployed there. The next investigation is the native VFS/installer read path, followed by the kernel-mediated store-write path. Preserve the publisher, attestation and StoreWrite checks while resolving these failures.

## Official NONOS distribution

For installation on stock official images, obtain a publisher certificate recognized by that distribution, with the Chat namespace and capability ceiling authorized. Re-sign the manifest and package with the corresponding publisher identity and satisfy the receiving kernel's attestation policy. The preview's private trust policy must not be silently installed as a substitute.

The repository remains private. Public marketplace publication or changing repository visibility is a separate release decision.
