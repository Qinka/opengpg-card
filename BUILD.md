# Building ESP32-S3 OpenPGP Card

## Important Prerequisites

This project targets the **ESP32-S3** microcontroller which uses the **Xtensa** architecture. The Xtensa architecture is **not supported** by the standard Rust toolchain. You **must** install the Espressif Rust toolchain.

## Step-by-Step Build Instructions

### 1. Install Espressif Rust Toolchain

```bash
# Install espup tool
cargo install espup

# Install the Espressif Rust toolchain
# This installs a custom Rust compiler with Xtensa support
espup install

# Add the environment to your shell
# This sets up PATH and other variables
. $HOME/export-esp.sh

# Optional: Add to your .bashrc or .zshrc
echo '. $HOME/export-esp.sh' >> ~/.bashrc
```

### 2. Verify Installation

```bash
# Check that the Xtensa target is available
rustup target list | grep xtensa

# You should see: xtensa-esp32s3-none-elf
```

### 3. Install Build Tools

```bash
# Install espflash for flashing firmware
cargo install espflash

# Install ldproxy (linker proxy)
cargo install ldproxy
```

### 4. Build the Project

```bash
# Navigate to project directory
cd opengpg-card

# Build in release mode (optimized for size)
cargo build --release

# Or build in debug mode (faster compilation)
cargo build
```

### 5. Flash to ESP32-S3

```bash
# Flash and monitor serial output
cargo run --release

# Or manually flash
espflash flash target/xtensa-esp32s3-none-elf/release/opengpg-card

# Monitor serial output
espflash monitor
```

## Troubleshooting

### Error: "xtensa-esp32s3-none-elf target not found"

**Solution:** You haven't installed the Espressif Rust toolchain. Follow Step 1 above.

### Error: "data-layout for target differs"

**Solution:** Your environment is using the standard Rust toolchain instead of the Espressif one. Run:
```bash
. $HOME/export-esp.sh
```

### Error: "espflash: command not found"

**Solution:** Install espflash:
```bash
cargo install espflash
```

### Error: "USB device not found"

**Solutions:**
- Ensure ESP32-S3 is connected via USB
- Check that you're using a data-capable USB cable
- Try a different USB port
- On Linux, check device permissions:
  ```bash
  sudo usermod -a -G dialout $USER
  # Log out and back in
  ```

### Build is very slow

The first build will be slow because it compiles many dependencies. Subsequent builds use incremental compilation and will be faster.

For faster development iterations:
- Use `cargo build` (debug mode) instead of `--release`
- Use `cargo check` for syntax checking without building

## CI/CD Considerations

For CI/CD pipelines, you need to:

1. Install the Espressif Rust toolchain in your CI environment
2. Source the environment variables
3. Use a Docker image with the toolchain pre-installed

Example GitHub Actions workflow:

```yaml
name: Build

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Install Espressif Rust
        run: |
          cargo install espup
          espup install
          . $HOME/export-esp.sh
          
      - name: Build
        run: |
          . $HOME/export-esp.sh
          cargo build --release
```

Or use the official Espressif container:
```yaml
    container:
      image: espressif/idf-rust:all_latest
```

## Alternative: Using Docker

If you prefer Docker, you can use the official Espressif image:

```bash
# Pull the image
docker pull espressif/idf-rust:all_latest

# Build the project
docker run --rm -v $(pwd):/project -w /project espressif/idf-rust:all_latest \
  cargo build --release

# Flash (requires USB device access)
docker run --rm -v $(pwd):/project -w /project --device=/dev/ttyUSB0 \
  espressif/idf-rust:all_latest \
  espflash flash target/xtensa-esp32s3-none-elf/release/opengpg-card
```

## Build Configuration

### Optimization Levels

The project is configured for size optimization in release mode:
- `opt-level = 'z'` - Optimize for size
- `lto = 'fat'` - Full link-time optimization
- `codegen-units = 1` - Better optimization (slower builds)

You can modify these in `Cargo.toml` under `[profile.release]`.

### Feature Flags

Currently, the project doesn't use feature flags, but you can add them to enable/disable functionality:

```toml
[features]
default = []
usb-logging = []
debug-crypto = []
```

## Memory Usage

The binary size and memory usage depend on enabled features:
- Flash usage: ~300KB - 500KB (with all features)
- RAM usage: ~100KB - 200KB (heap + stack + static)

Check with:
```bash
cargo size --release -- -A
```

## Next Steps

After successful build:
1. Flash to your ESP32-S3 device
2. Connect via USB
3. Test with GnuPG (see main README.md)
4. Monitor logs via serial console

## Resources

- [ESP-RS Book](https://esp-rs.github.io/book/)
- [ESP32-S3 Datasheet](https://www.espressif.com/sites/default/files/documentation/esp32-s3_datasheet_en.pdf)
- [Rust Embedded Book](https://docs.rust-embedded.org/book/)
- [espup Documentation](https://github.com/esp-rs/espup)
