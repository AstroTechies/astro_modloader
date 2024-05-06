# Astroneer Modloader

A modloader for Astroneer, rewritten in Rust.

## Installation

### Windows

Download the modloader (`astro_modloader.exe`) from the [releases
page](https://github.com/AstroTechies/astro_modloader/releases/latest), below the changelog.

### Linux

Pre-built binaries are not yet dsitributed for Linux. To build the modloader yourself on Linux:

- Use your distribution's package manager to install `rustup`, `git`, and `build-essential`,
- Use `rustup` to install the Rust programming language,
- Use `cargo` to install `cargo-about`,
- Use `git` to clone the modloader's repository,
- Then run the following commands in the root of the repository:

```
export USE_PRECOMPILED_CPP_LOADER=1
export USE_PREBUILT_ASSETS=1
cargo build --release
```

The last command may take a while to run. Once it's done, the executable (`astro_modloader`) will be
in `target/release`.
