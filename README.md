# astro_modloader (Rust)

A mod loader for Astroneer, rewritten in Rust.

## Installation

### Windows

Download the mod loader (`astro_modloader.exe`) from the [releases
page](https://github.com/AstroTechies/astro_modloader/releases/latest), below the changelog.

### Linux

Download the mod loader (`astro_modloader-linux-x64`) from the [releases
page](https://github.com/AstroTechies/astro_modloader/releases/latest), below the changelog. If the published binary is not compatible with your system, please follow the Compilation Guide below.

#### Compilation Guide
If you would like to build the mod loader yourself on Linux, follow the steps below:

- Use your distribution's package manager to install `rustup` and `git`,
- Use `rustup` to install the Rust programming language,
- If `rustup` didn't install it for you, install `build-essential` or your distro's equivalent,
- Use `cargo` to install `cargo-about`,
- Use `git` to clone the mod loader's repository,
- Then run the following commands in the root of the repository:

```
export USE_PRECOMPILED_CPP_LOADER=1
export USE_PREBUILT_ASSETS=1
cargo build --release
```

The last command may take a while to run. Once it's done, the executable (`astro_modloader`) will be
in `target/release`.
