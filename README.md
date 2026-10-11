# RustyNES

A Nintendo Entertainment System (NES) emulator written in Rust, currently focused on CPU emulation.

The 6502 CPU core implements the official instruction set and all addressing modes, enough to run a first program: the classic Snake demo, rendered with SDL3. The project is an ongoing exploration of instruction execution, memory access and the inner workings of a classic gaming system.

---

# How to run

## Requirements

* [Rust](https://rustup.rs) 1.80 or newer
* SDL3 available on your system (see the [`sdl3` crate docs](https://docs.rs/sdl3) for installation and build-from-source options)

## Run

```
git clone https://github.com/jpedrosmenezes/RustyNES.git
cd RustyNES
cargo run --release
```

This launches the Snake demo.

| Key | Action |
|-----|--------|
| `W` `A` `S` `D` | Move |
| `Esc` | Quit |

---

# Project structure

```
src/
├── main.rs      # SDL3 frontend and Snake demo runner
├── cpu.rs       # 6502 CPU: registers, flags, addressing modes, instructions
└── opcodes.rs   # Opcode table (mnemonic, length, cycles, addressing mode)
```

---

# Credits

For learning how to build this I used the information/code/insights from:

* *[Writing NES Emulator in Rust](https://bugzmanov.github.io/nes_ebook/)*: base structure and the Snake demo program
* *[6502 Instruction Set](https://www.masswerk.at/6502/6502_instruction_set.html)*
* *[NES Emulator From Scratch](https://www.youtube.com/playlist?list=PLrOv9FMX8xJHqMvSGB_9G9nZZ_4IgteYf)*
* *[Emulation Online: Nintendo® Entertainment System (NES)](https://www.emulationonline.com/systems/nes/)*

---

# Development Progress

- [x] CPU official instruction set
- [x] Memory addressing modes
- [x] First program execution milestone (Snake demo)
- [ ] CPU validation with `nestest`
- [ ] Interrupts (BRK, NMI, IRQ)
- [ ] Cycle counting
- [ ] Unofficial opcodes
- [ ] Bus emulation
- [ ] Cartridge support (iNES, mappers)
- [ ] Picture Processing Unit (PPU)
- [ ] Joypad input
- [ ] PPU scrolling
- [ ] Audio Processing Unit (APU)

This project is under active development. The checklist reflects the current implementation status and will be updated as new components are completed.

---

# License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.