# Security Policy

## Supported versions

Security fixes are released for the latest `0.1.x` version of `buildah-ffi` and `oci-builder` on crates.io. Those two crates share one version number. Older releases, including earlier `0.1` releases, are not maintained.

The supported Buildah pin is the `go.podman.io/buildah` version required by `crates/buildah-ffi/shim/go.mod`. A fix may bump that pin.

## Reporting a vulnerability

Report vulnerabilities privately through [GitHub private vulnerability reporting](https://github.com/geoffsee/oci-builder/security/advisories/new).

Include:

- The crate and version (`buildah-ffi`, `oci-builder`, or both)
- Whether you used the library API or the `oci-builder` CLI
- The host (distribution, root or rootless, storage driver, isolation)
- What an attacker can do, and a way to reproduce it

Do not open a public issue or pull request for an unfixed vulnerability.

You will get an acknowledgement within 7 days, and a decision on whether the report is accepted. An accepted report is fixed on a private branch and released as a new `0.1.x` version. You will be told when that release is published. A declined report gets the reason.

A GitHub Security Advisory is published with the fix. A CVE is requested when the impact warrants one.

There is no bug bounty.

## Scope

In scope:

- `buildah-ffi`, the Go shim under `crates/buildah-ffi/shim`, and the `oci-builder` CLI
- Container breakout, unexpected code execution, credential or password disclosure, and privilege escalation caused by this code
- A Buildah or containers-library flaw that affects the pinned module version
- Compromise of the release path (the `release.yml` workflow or the crates.io trusted publisher)

Out of scope:

- A Containerfile, Git repository, or registry that the caller chose to build, when the engine behaves as documented
- Host configuration, including user namespaces, a signature policy of `insecureAcceptAnything`, or choosing to run as root
- runc, crun, or kernel bugs that this project does not introduce
- Buildah issues that do not affect the pinned module version. Report those to [Buildah](https://github.com/containers/buildah/security)
