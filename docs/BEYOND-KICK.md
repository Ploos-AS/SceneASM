# SceneASM: path beyond Kick Assembler

SceneASM is not a Java-free clone. The project aims to become the scene-first native toolchain people choose because it gives them more control over the machine and a better feedback loop.

## Product pillars

1. **Hardware truth first** — timing, addressing quirks, undocumented instructions, memory layout and machine-specific chips are modeled explicitly.
2. **Tooling is part of the language** — assembler metadata powers LSP hover, diagnostics, completion, symbol navigation, cycle views and later raster visualization.
3. **Fast native workflow** — one native Rust toolchain, deterministic builds, watch mode and direct emulator launch.
4. **Scene workflow first** — cycle assertions, raster-aware analysis, memory maps, assets, packing and profiling are first-class rather than external glue.
5. **Migration without stagnation** — provide practical Kick Assembler migration/compatibility tooling, but do not preserve design choices that block a better language.
6. **Family resemblance, hardware specialization** — future Amiga and K16 SceneASM editions reuse concepts and UX where useful while keeping CPU/chipset semantics target-native.

## Competitive bar

Before a 1.0 claim, SceneASM should cover the workflows that make Kick Assembler attractive to serious C64 users and add capabilities that are difficult to bolt onto a traditional assembler:

- complete 6502/6510 addressing and expression system
- macros, scopes/namespaces, structs/data and compile-time facilities
- documented and undocumented opcode control
- relocatable segments and strong memory-map diagnostics
- cycle metadata, variable-cycle diagnostics and cycle assertions
- PAL/NTSC raster-aware analysis
- C64 symbols/register knowledge in the language server
- excellent VSCodium/VS Code extension
- source-level VICE launch/debug integration
- asset pipeline hooks with reproducible dependency tracking
- PRG, D64 and CRT-oriented workflows
- machine-readable build/debug metadata
- formatter and migration tooling
- reproducible builds and regression corpus

## Rule for shared architecture

Do not create a lowest-common-denominator IR merely so C64, Amiga and K16 can share code. Share infrastructure when semantics genuinely match. Hardware-specific behavior belongs in hardware-specific models.
