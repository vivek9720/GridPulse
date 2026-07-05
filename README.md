# GridPulse

GridPulse is a Rust library and fuzzing target suite for an offline smart-grid
mesh telemetry decoder. It models utility field traffic such as meter
last-gasp outage reports, feeder route advertisements, template-based interval
readings, and compact crew-dispatch scripts.

The parser is multi-stage:

1. Binary frame synchronization and optional CRC validation.
2. Mesh fragmentation and reassembly.
3. Session lifecycle negotiation.
4. Template dictionary installation and compaction.
5. Template-driven measurement decoding.
6. Feeder route dictionary updates.
7. Nested crew-script decoding.

The repository is built for ClusterFuzzLite-style intake:

- Rust library with no network dependencies.
- cargo-fuzz-compatible harnesses under `fuzz/`.
- `.clusterfuzzlite/build.sh` builds every harness into `$OUT`.
- Per-target seed corpora live under `fuzz/corpus/`.

## Local Layout

```text
GridPulse/
  src/
  fuzz/
    Cargo.toml
    fuzz_targets/
    corpus/
    dictionary.txt
  .clusterfuzzlite/
    build.sh
    project.yaml
```

## Harnesses

- `frame_fuzzer`: decodes one byte stream as mesh frames.
- `replay_fuzzer`: exercises replay mode with staged fragmentation and state.
- `script_fuzzer`: decodes crew dispatch scripts directly.

The build is hermetic: all dependencies are path dependencies inside the repo.
