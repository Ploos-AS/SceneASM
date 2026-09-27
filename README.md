# SceneASM

SceneASM is a native, scene-first assembler/toolchain written in Rust.

The first target is the Commodore 64 / MOS 6510. The long-term family may include Amiga and K16 variants, but hardware specialization is a design requirement: shared UX and tooling must never flatten machine-specific capabilities.

## M0 goals

- Native Rust implementation; no JVM dependency.
- C64/6510 first-class target.
- Deterministic assembly and PRG output.
- Clear diagnostics suitable for editor/LSP use.
- Architecture that can grow into cycle/raster analysis, memory maps and scene-specific tooling.
- VSCodium/VS Code extension backed by an LSP, not a syntax-highlighting-only plugin.

## Current M0 bootstrap

```sh
cargo test
cargo run -p sceneasm -- check examples/minimal.asm
cargo run -p sceneasm -- build examples/minimal.asm
```

Supported so far: `.org`, labels, `.byte`, and a deliberately tiny first instruction subset (`SEI`, `CLI`, `NOP`, `RTS`, `BRK`, immediate `LDA`, absolute `STA`, absolute `JMP`). M0 will expand this to the complete NMOS 6502/6510 instruction/addressing model.

## Design rule: familiar, not generic

SceneASM should feel familiar across machines while remaining native to each machine.

Shared where useful:

- command vocabulary and project layout
- expression language and macro concepts
- diagnostics conventions
- LSP protocol and editor UX
- symbol/debug metadata
- build/run/profile workflow

Hardware-specific by design:

- instruction set and addressing modes
- executable/object formats
- memory models
- cycle and contention rules
- raster/video/audio concepts
- coprocessors and DMA
- emulator/debugger integration

A future Amiga SceneASM should therefore understand 68000-family and Amiga custom-chip realities; a future K16 SceneASM should expose K16-specific CPU/coprocessor features. Compatibility of muscle memory is a feature. Lowest-common-denominator abstraction is not.

## License

Software source is MIT licensed unless stated otherwise.
