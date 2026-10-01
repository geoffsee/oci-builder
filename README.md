# buildah-ffi

Rust library and `oci-builder` CLI that embed [Buildah](https://github.com/podman-container-tools/buildah) in-process. There is no Buildah daemon and no `buildah` binary on `PATH`.

Linux runs the engine in-process. On macOS the same API boots a small Linux guest through Apple's Virtualization framework and runs that engine inside it. Other targets link a stub that returns `unsupported`.

## Install from crates.io

Library:

```bash
cargo add buildah-ffi
```

```toml
buildah-ffi = "0.1"
```

CLI:

```bash
cargo install oci-builder
```

Both commands compile the crate on your machine. On Linux, `build.rs` builds the Buildah Go shim and links it into the binary, so the build needs Go 1.26 or newer, a C compiler, and pkg-config. libseccomp is optional: when pkg-config finds it, the seccomp build tag is turned on. On macOS the CLI and library boot a Linux guest instead of linking Buildah into the Mac process. Build that guest on Linux with [`guest/build.sh`](guest/build.sh) and leave `guest/out/` next to the source tree, or point `ROB_GUEST_KERNEL` and `ROB_GUEST_INITRD` at the two files. The macOS build embeds those files with `include_bytes!`. The first boot writes them to `~/Library/Caches/oci-builder`. A build without them still looks for `guest/out` at runtime. `cargo run` and `cargo test` ad-hoc sign the binary with the `com.apple.security.virtualization` entitlement. `cargo build` on macOS signs through the `rustc` wrapper in [`.cargo/config.toml`](.cargo/config.toml). Other non-Linux hosts link a stub. `startup()` and `oci-builder` then exit with code 8 and a message that Buildah is available on Linux only.

An image build also needs the host prerequisites under [Build requirements](#build-requirements). Rootless use needs user namespaces. A pull needs a signature policy. A `RUN` instruction with `oci` or `rootless` isolation needs runc or crun on `PATH`. `chroot` isolation and a scratch image that only uses `COPY` do not.

## Library

Call `startup()` before spawning threads or parsing arguments. Buildah may re-exec this process for a rootless user namespace, and that child has to reach `startup` before anything else reads `argv`.

```rust
use buildah_ffi::{startup, BuildRequest, Builder, Config, StorageDriver};

fn main() -> Result<(), buildah_ffi::Error> {
    startup()?;
    let builder = Builder::open(Config {
        storage_driver: Some(StorageDriver::Vfs),
        ..Config::default()
    })?;
    let info = builder.build(BuildRequest::new("Dockerfile", "."))?;
    println!("image_id={}", info.image_id);
    builder.shutdown()?;
    Ok(())
}
```

`shutdown()` releases the storage mounts. Dropping the `Builder` does not.

## CLI

```bash
oci-builder diagnose

oci-builder --root /tmp/graph --runroot /tmp/run --storage-driver vfs \
  --signature-policy policy.json \
  build -f Dockerfile -t localhost/app:latest --pull never --isolation chroot

oci-builder --root /tmp/graph --runroot /tmp/run --storage-driver vfs \
  push localhost/app:latest localhost:5000/app:latest --insecure
```

`policy.json` for a local store that does not verify signatures:

```json
{"default":[{"type":"insecureAcceptAnything"}]}
```

Stdout of `build` and `push` is `image_id=`, `digest=`, and `reference=`. Logs go to stderr. The exit code is the `ErrorCode` value. Clap's own usage errors stay exit code 2. `diagnose` prints the prerequisite report and exits 0 when the host is ready.

Podman uses its own store. Export the tag to a `docker-archive`, then load that file. Pass the same `--root`, `--runroot`, `--storage-driver`, and `--signature-policy` as the build:

```bash
oci-builder --root /tmp/graph --runroot /tmp/run --storage-driver vfs \
  --signature-policy policy.json \
  push localhost/app:latest \
  docker-archive:/tmp/app.tar:localhost/app:latest

podman load -i /tmp/app.tar
```

On macOS the push runs in the guest. A destination such as `/tmp/app.tar` is inside that guest and is gone when the VM exits. `/mnt/policy` is the virtiofs share of the directory that contains `--signature-policy`. With the policy at `/tmp/rob/policy.json`, this guest path leaves `/tmp/rob/app.tar` on the Mac:

```bash
oci-builder --root /tmp/rob/graph --runroot /tmp/rob/run --storage-driver vfs \
  --signature-policy /tmp/rob/policy.json \
  push localhost/app:latest \
  docker-archive:/mnt/policy/app.tar:localhost/app:latest

podman load -i /tmp/rob/app.tar
```

[`examples/scratch-copy`](examples/scratch-copy) is `FROM scratch` and contains only `/hello.txt`, so the loaded image has no command to start. Copy the file out. `podman cp` to stdout writes a tar archive, and `tar -xO` prints `hello`:

```bash
podman create --name scratch-copy --entrypoint /hello.txt localhost/scratch-copy:latest
podman cp scratch-copy:/hello.txt - | tar -xO
podman rm scratch-copy
```

The sample Dockerfiles in [`examples/`](examples/) live in the source repository. They are not part of the published crates.

## Architecture

```
oci-builder  -->  buildah-ffi (safe Rust)
                      |
                      | C ABI (rob_*.h): borrowed C strings in, malloc'd buffers out
                      v
                 Go c-archive
                      |
                      +-- buildah.InitReexec / rootless re-exec
                      +-- containers/storage (one store per process)
                      +-- imagebuildah.BuildDockerfiles
                      +-- libimage tag
                      +-- buildah.Push
```

Buildah-specific types stay behind the C ABI. The Rust API (`Builder::build`, `Builder::push`, `Builder::tag`) does not expose Go structs, so Buildah's internals can change without an ABI break.

The archive is linked with `static:+whole-archive` so the Go runtime's `.init_array` constructor is not dropped. `startup` is a normal call from `main`. A Rust constructor would race that Go constructor and can deadlock inside cgo.

## Safety

- Call `startup()` as the first thing in `main`, before spawning threads and before parsing arguments. Storage helpers re-exec this binary and dispatch on `argv[0]`. If `startup` has not run, those children do not reach their handlers.
- Rootless startup may re-exec. The parent waits and exits with the child's status. The child's `argv[0]` is the original name plus `-in-a-user-namespace`, and the rest of the arguments are unchanged. `startup` returns only in the child (or when no re-exec was required).
- Do not call `unshare` or `startup` while holding a lock you expect to survive a fork. The re-exec path runs `/proc/self/exe`, then the parent exits.
- Input strings are borrowed for the call. The shim copies them with `C.GoString`.
- Output buffers are `malloc`'d, NUL-terminated, and owned by the caller. `len` does not include the NUL. A NULL pointer with `len == 0` is empty. The safe API frees them before returning.
- `rob_buildah_version` is process-lifetime storage. Do not free it.
- Log callbacks run on a Go thread. The bytes are valid only for that call; the safe API copies them first. The callback must not call back into `Builder` (the engine lock is held) and must not unwind. Panics in the callback are caught and discarded.
- Cancel tokens are integers, not Go pointers. `cancel` does not take the engine lock. `Drop` frees the token and does not cancel. A token that is already freed when an operation starts is reported as cancelled, so the token has to outlive the call. `Clone` shares one token.
- One store is open per process. `Builder` is not cloned. `Drop` does not shut the store down; call `shutdown` so the graph driver can release mounts. A second `open` fails until `shutdown` returns.
- Passwords are copied into the Go heap for the push and are not written to logs. `Debug` for `PushRequest` redacts the password.
- The Go runtime installs its own signal handlers. Cancellation is the supported way to interrupt a build or push.

`MaybeReexecUsingUserNamespace` can still `os.Exit` if user-namespace setup fails after the pre-check. `startup` checks `newuidmap`, `newgidmap`, subordinate IDs, and the user-namespace sysctls first so the common failures return `ErrorCode::Prerequisite` instead of exiting.

## Build requirements

| Requirement | Why |
| --- | --- |
| Linux, 64-bit | The engine uses cgo, namespaces, and `/proc/self/exe`. |
| Go **>= 1.26** | Buildah v1.45.1 sets `go 1.26.0`. |
| gcc, libc headers, pkg-config | cgo. |
| libseccomp (optional) | Enables the `seccomp` tag. Without it, `RUN` that installs a seccomp profile fails. The library is linked dynamically when the tag is on. |
| libapparmor (optional) | Enables the `apparmor` tag when pkg-config finds it. |
| uidmap (`newuidmap`, `newgidmap`) | Rootless user namespaces. The binaries must be setuid or carry the matching file capability. |
| `/etc/subuid` and `/etc/subgid` | Subordinate ranges for the calling user. |
| fuse-overlayfs | Rootless overlay. `vfs` needs neither overlay nor a mount helper. |
| runc or crun | `RUN` with `oci` or `rootless` isolation. `chroot`, and a scratch image that only uses `COPY`, do not need them. |
| `policy.json` | Pulls fail closed when neither `/etc/containers/policy.json` nor `~/.config/containers/policy.json` exists. |

`startup` / `oci-builder diagnose` prints `status: ready` or `status: blocked` with `[ok]`, `[warn]`, and `[fail]` lines. Blocked startup returns exit code 4 and includes that report. Missing overlay, a missing policy, and a missing runc are warnings. A missing `newuidmap`, a zero `user.max_user_namespaces`, `kernel.unprivileged_userns_clone=0`, or `kernel.apparmor_restrict_unprivileged_userns=1` is a failure for non-root.

Build tags always include `exclude_graphdriver_btrfs`, `exclude_graphdriver_devicemapper`, and `containers_image_openpgp` (pure-Go signature verification). Image signing is not supported. Extra tags can be added with `ROB_GO_TAGS`.

Cross-compiling the archive is refused unless `CC` and `ROB_ALLOW_CROSS=1` are set. `build.rs` runs `go build -mod=readonly`, so `crates/buildah-ffi/shim/go.sum` has to be present. Refresh it on Linux (or in a Linux container) with `GOOS=linux`:

```bash
cd crates/buildah-ffi/shim
go mod tidy
```

## Versions

Direct module requirement:

- `go.podman.io/buildah` **v1.45.1** (module `go.podman.io/buildah`, repository `podman-container-tools/buildah`)

Versions recorded in `crates/buildah-ffi/shim/go.mod` after `go mod tidy` on Linux:

- `go.podman.io/storage` v1.64.1
- `go.podman.io/image/v5` v5.41.2
- `go.podman.io/common` v0.69.2

Older `github.com/containers/buildah` 1.43.x releases are affected by GO-2026-5116 (build breakout via a malicious Containerfile or Git HTTP server). This tree tracks v1.45.1, not that line.

`go.sum` is the lock. If `go mod tidy` moves a transitive pin, update this list to match.

## Limitations

- On macOS the engine runs in a Linux guest. `startup()` does not boot it, so `--help` works. The guest starts on the first build, tag, or push. The kernel and initramfs come from `guest/build.sh` on a Linux machine of the same architecture and are embedded in the macOS binary when they are present at compile time. `RUN` with `oci` isolation still needs `runc` or `crun` in that guest; the first image does not include them. Scratch and `COPY` builds use `vfs` and `chroot`. Other non-Linux hosts link the stub and exit 8.
- `RUN` under `oci` or `rootless` isolation needs runc or crun on `PATH`. They are not linked in.
- Signing is not implemented. Verification uses the pure-Go OpenPGP tag.
- One store per process.
- The Go runtime takes signals such as `SIGURG`. Cancel through `CancelToken` instead of relying on a Rust `ctrl-c` hook inside a build.
- Rootless re-exec changes `argv[0]` and the original process does not continue after the child exits.
- When the `seccomp` tag is enabled, `libseccomp` is a dynamic runtime dependency.
- `cargo test` of the library does not drive the engine. The test harness is already multithreaded, so a Buildah re-exec from inside it would restart the harness. `crates/oci-builder/tests/build_image.rs` runs the `oci-builder` binary. It builds `FROM scratch` plus `COPY` with the `vfs` driver and `chroot` isolation, and it pushes only when `ROB_TEST_REGISTRY` is set (HTTP, TLS verification skipped).

## Release

Pushing a tag `vX.Y.Z` runs [`.github/workflows/release.yml`](.github/workflows/release.yml). The workflow runs the CI tests, publishes `rob-proto`, `buildah-ffi`, and `oci-builder` with [crates.io trusted publishing](https://crates.io/docs/trusted-publishing), and attaches binary archives to the GitHub release. The tag must match `workspace.package.version`.

| Archive | Where it is built |
| --- | --- |
| `oci-builder-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` | Ubuntu 24.04 |
| `oci-builder-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz` | Ubuntu 24.04 on arm64 |
| `oci-builder-vX.Y.Z-aarch64-apple-darwin.tar.gz` | macOS on Apple Silicon |

Each archive contains the `oci-builder` binary. Linux binaries are linked on Ubuntu 24.04 and need that glibc plus `libseccomp.so.2`. The Apple Silicon archive is only the binary: the arm64 Linux job builds the guest, and the macOS build embeds `vmlinuz` and `initramfs` into it. The first run writes those images to `~/Library/Caches/oci-builder`.

For each crate, the GitHub trusted publisher is workflow filename `release.yml` and environment `release`. Trusted publishing updates a crate that already exists, so the first upload of a crate still uses an API token. Later versions do not store a crates.io token in this repository.

## License

Apache-2.0, the same license as Buildah. See [LICENSE](LICENSE).
