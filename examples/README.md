# Example build contexts

Each subdirectory is a context for `oci-builder build`. The successful ones use `FROM scratch`, so they do not pull a base image and do not need runc or crun. On Linux:

```bash
oci-builder \
  --root /tmp/rob-graph --runroot /tmp/rob-run --storage-driver vfs \
  --signature-policy /tmp/policy.json \
  build --context examples/scratch-copy -t localhost/scratch-copy:latest \
  --pull never --isolation chroot
```

`/tmp/policy.json` for a local store that does not verify signatures:

```json
{"default":[{"type":"insecureAcceptAnything"}]}
```

| Directory | What it exercises | Expected result |
| --- | --- | --- |
| `scratch-copy` | `COPY` of one file onto `scratch` | image id and digest |
| `build-args` | global `ARG`, `ENV`, `LABEL`, `WORKDIR`, relative `COPY` | pass `--build-arg GREETING=world` |
| `multi-stage` | named stages and `COPY --from` | default stage is `final`; `--target docs` builds the first stage; `--target missing` fails |
| `containerfile` | `Containerfile` with no `Dockerfile` | omit `-f` and let the CLI discover it |
| `dockerignore` | `COPY .` with an ignore file | context includes files the ignore list drops |
| `missing-copy` | `COPY` of a path that is not in the context | exit 5 |
| `unknown-instruction` | an instruction Buildah does not recognize | exit 5 |

`crates/oci-builder/tests/build_image.rs` builds this table when the engine is available. macOS links the stub and skips them.
