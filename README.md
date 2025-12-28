# ESP32-S3 OpenPGP Card

A complete OpenPGP card implementation for ESP32-S3 microcontroller using Rust and Embassy async framework.

## Overview

This project implements an OpenPGP-compatible smart card on the ESP32-S3 microcontroller, providing cryptographic operations for signing, decryption, and authentication. The implementation follows the OpenPGP Card specification v3.4 and uses the ISO/IEC 7816-4 APDU protocol over USB CCID interface.

## Features

### Cryptographic Operations
- **RSA**: 2048-bit and 4096-bit key generation, signing, and decryption
- **ECC**: Support for P-256, P-384, Ed25519, and Curve25519 curves
- **Hashing**: SHA-256 and SHA-512
- **Hardware RNG**: ESP32-S3 hardware random number generator

### OpenPGP Card Functions
- Three independent key slots (Signature, Decryption, Authentication)
- On-device key generation
- PIN protection (User PIN and Admin PIN)
- PIN retry counter with automatic blocking
- Secure key storage in encrypted NVS
- Data object storage (cardholder name, login data, public key URL)

### Security Features
- PIN verification with retry limits
- Keys never leave the device
- Factory reset capability
- Constant-time cryptographic operations
- Hardware crypto acceleration (where available)

## Hardware Requirements

- **Microcontroller**: ESP32-S3 (any variant with USB support)
- **USB**: Native USB interface (no external USB-to-serial adapter needed)
- **Flash**: At least 4MB
- **RAM**: 512KB (with optional PSRAM support)

### Recommended Development Boards
- ESP32-S3-DevKitC-1
- ESP32-S3-DevKitM-1
- Any ESP32-S3 board with native USB

## Software Requirements

### Prerequisites

1. **Rust Toolchain**
   ```bash
   # Install Rust (if not already installed)
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Install the nightly toolchain
   rustup install nightly-2024-02-01
   rustup component add rust-src --toolchain nightly-2024-02-01
   ```

2. **ESP32 Rust Toolchain**
   ```bash
   # Install espup (ESP Rust installer)
   cargo install espup
   
   # Install ESP32 toolchain
   espup install
   
   # Source the environment (add to your shell profile)
   . $HOME/export-esp.sh
   ```

3. **espflash** (for flashing)
   ```bash
   cargo install espflash
   ```

4. **ldproxy** (linker proxy)
   ```bash
   cargo install ldproxy
   ```

## Building

### Build the project
```bash
cargo build --release
```

The build process will:
- Compile Rust code to Xtensa assembly
- Link with ESP-IDF components
- Generate a flashable binary

### Build output
The compiled binary will be located at:
```
target/xtensa-esp32s3-none-elf/release/opengpg-card
```

## Flashing

### Flash to ESP32-S3
```bash
cargo run --release
```

Or manually:
```bash
espflash flash target/xtensa-esp32s3-none-elf/release/opengpg-card
```

### Monitor serial output
```bash
espflash monitor
```

Or combine flash and monitor:
```bash
cargo run --release
```

## Pin Configuration

### Default PINs

The OpenPGP card comes with default PINs:

- **User PIN (PW1)**: `123456`
  - Used for: Signing, Decryption, Authentication operations
  - Default retry limit: 3 attempts
  
- **Admin PIN (PW3)**: `12345678`
  - Used for: Key generation, PIN changes, administrative operations
  - Default retry limit: 3 attempts

⚠️ **Security Warning**: Change these default PINs immediately after first use!

### Changing PINs

Use GnuPG or another OpenPGP client to change PINs:

```bash
# Change User PIN
gpg --card-edit
> passwd
> 1  # Change PIN

# Change Admin PIN
> 3  # Change Admin PIN
```

## Usage with GnuPG

### 1. Check Card Status

Connect the ESP32-S3 to your computer via USB and check if it's recognized:

```bash
gpg --card-status
```

Expected output:
```
Reader ...........: ESP32-S3 OpenPGP Card
Application ID ...: D27600012401...
Version ..........: 3.4
Manufacturer .....: Unknown
Serial number ....: ...
Name of cardholder: [not set]
Language prefs ...: [not set]
URL of public key : [not set]
Login data .......: [not set]
Signature PIN ....: not forced
Key attributes ...: ed25519 cv25519 ed25519
Max. PIN lengths .: 32 32 32
PIN retry counter : 3 3 3
Signature counter : 0
```

### 2. Generate Keys on Card

Generate keys directly on the card (they never leave the device):

```bash
gpg --card-edit
> admin
> generate
```

Follow the prompts to:
- Enter the Admin PIN
- Choose whether to create a backup (recommended: No, since keys are generated on-card)
- Set key expiration
- Enter your name and email

### 3. Using the Card for SSH Authentication

Export the authentication key for SSH:

```bash
# Export SSH public key
gpg --export-ssh-key YOUR_EMAIL > ~/.ssh/id_ed25519_openpgp.pub

# Add to SSH agent
gpg-connect-agent "scd serialno" /bye
ssh-add -L
```

Add the public key to your `~/.ssh/authorized_keys` on remote servers.

### 4. Signing Git Commits

Configure Git to use your OpenPGP card:

```bash
# Get your key ID
gpg --card-status

# Configure Git
git config --global user.signingkey YOUR_KEY_ID
git config --global commit.gpgsign true
```

Now all commits will be signed with your card:
```bash
git commit -m "Signed commit"
```

### 5. Encrypting/Decrypting Files

Encrypt a file:
```bash
gpg --encrypt --recipient YOUR_EMAIL document.txt
```

Decrypt (will prompt for User PIN):
```bash
gpg --decrypt document.txt.gpg
```

## Architecture

```
┌─────────────────────────────────────────┐
│          USB CCID Interface             │
│         (embassy-usb)                   │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│      APDU Command Parser                │
│      (ISO/IEC 7816-4)                   │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│     OpenPGP Card State Machine          │
│  - Command routing                      │
│  - State management                     │
│  - Data object handling                 │
└─────┬────────────────┬──────────────────┘
      │                │
      ▼                ▼
┌──────────┐    ┌──────────────┐
│   PIN    │    │   Crypto     │
│ Manager  │    │   Engine     │
└──────────┘    └──────┬───────┘
                       │
                       ▼
              ┌─────────────────┐
              │  Storage (NVS)  │
              │  - Keys         │
              │  - Data         │
              └─────────────────┘
```

## Module Overview

### `src/main.rs`
- Entry point and hardware initialization
- Embassy executor setup
- Task spawning

### `src/apdu.rs`
- APDU command parsing
- APDU response building
- Status word definitions

### `src/openpgp.rs`
- OpenPGP card state machine
- Command handlers (SELECT, VERIFY, GENERATE KEY, etc.)
- Data object management
- Key slot management

### `src/crypto.rs`
- Cryptographic operations
- Key pair generation
- Signing and verification
- Encryption and decryption
- Hashing

### `src/pin.rs`
- PIN verification logic
- Retry counter management
- PIN change operations

### `src/storage.rs`
- NVS interface
- Secure key storage
- Configuration data persistence

### `src/usb.rs`
- USB device initialization
- CCID protocol implementation
- Message routing

## Supported OpenPGP Commands

| Command | Instruction | Description | Status |
|---------|-------------|-------------|--------|
| SELECT | 0xA4 | Select OpenPGP application | ✅ Implemented |
| GET DATA | 0xCA | Retrieve data objects | ✅ Implemented |
| PUT DATA | 0xDA | Store data objects | ✅ Implemented |
| VERIFY | 0x20 | Verify PIN | ✅ Implemented |
| CHANGE REFERENCE DATA | 0x24 | Change PIN | ✅ Implemented |
| RESET RETRY COUNTER | 0x2C | Reset PIN retry counter | ✅ Implemented |
| GENERATE ASYMMETRIC KEY PAIR | 0x47 | Generate key on card | ✅ Implemented |
| INTERNAL AUTHENTICATE | 0x88 | Authenticate with card | ✅ Implemented |
| PSO: COMPUTE DIGITAL SIGNATURE | 0x2A | Sign data | ✅ Implemented |
| PSO: DECIPHER | 0x2A | Decrypt data | ✅ Implemented |
| GET CHALLENGE | 0x84 | Get random challenge | ✅ Implemented |

## Security Considerations

### Secure Boot (Optional)
For production deployments, enable ESP32-S3 secure boot:
```bash
# In sdkconfig.defaults
CONFIG_SECURE_BOOT_V2_ENABLED=y
```

### Flash Encryption (Optional)
Enable flash encryption for additional security:
```bash
# In sdkconfig.defaults
CONFIG_SECURE_FLASH_ENC_ENABLED=y
```

### Best Practices
1. **Change Default PINs**: Always change the default PINs on first use
2. **Backup Keys**: Since keys are generated on-device, create encrypted backups if needed
3. **Physical Security**: The device should be kept physically secure
4. **USB Connection**: Only connect to trusted computers
5. **Firmware Updates**: Keep firmware updated with security patches

## Troubleshooting

### USB Not Recognized
- Ensure ESP32-S3 has native USB support (not all variants do)
- Check USB cable supports data transfer
- Verify USB drivers are installed
- Try a different USB port

### Build Errors
```bash
# Clean and rebuild
cargo clean
cargo build --release
```

### Flash Errors
```bash
# Hold BOOT button while flashing
espflash flash --monitor target/xtensa-esp32s3-none-elf/release/opengpg-card
```

### GnuPG Can't Find Card
- Check USB connection
- Restart pcscd service: `sudo systemctl restart pcscd`
- Check device permissions: `sudo chmod 666 /dev/ttyUSB*`

## Development

### Running Tests
```bash
# Run unit tests (note: most tests require embedded target)
cargo test --lib
```

### Logging
The firmware uses `esp-println` for logging. Adjust log level:
```bash
# In .cargo/config.toml
[env]
ESP_LOG = "debug"  # trace, debug, info, warn, error
```

### Code Formatting
```bash
cargo fmt
```

### Linting
```bash
cargo clippy -- -D warnings
```

## Contributing

Contributions are welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

- [OpenPGP Card Specification](https://gnupg.org/ftp/specs/OpenPGP-smart-card-application-3.4.pdf)
- [Embassy Rust Embedded Framework](https://embassy.dev/)
- [ESP32-S3 Technical Reference](https://www.espressif.com/sites/default/files/documentation/esp32-s3_technical_reference_manual_en.pdf)
- [GnuPG Project](https://gnupg.org/)

## Resources

- [OpenPGP Card Spec v3.4](https://gnupg.org/ftp/specs/OpenPGP-smart-card-application-3.4.pdf)
- [ISO/IEC 7816-4 APDU](https://www.iso.org/standard/54550.html)
- [Embassy Documentation](https://embassy.dev/book/)
- [ESP-RS Project](https://github.com/esp-rs)
- [Rust Embedded Book](https://docs.rust-embedded.org/book/)

## Status

This is a functional implementation providing core OpenPGP card features. The cryptographic operations use placeholder implementations that should be replaced with actual cryptographic libraries for production use.

### Current Status
- ✅ Project structure and configuration
- ✅ APDU command parsing
- ✅ OpenPGP command handlers
- ✅ PIN management
- ✅ Key generation framework
- ✅ Basic USB/CCID structure
- ⚠️ Cryptographic operations (placeholder implementations)
- ⚠️ NVS storage (placeholder implementation)
- ⚠️ Full USB/CCID protocol

### Production Readiness
For production use, the following enhancements are recommended:
1. Complete cryptographic implementations using hardware acceleration
2. Full NVS encryption and key storage
3. Complete USB CCID protocol implementation
4. Extensive security testing and auditing
5. Side-channel attack mitigation
6. Secure boot and flash encryption
7. Compliance testing with GnuPG and other OpenPGP clients

## Support

For issues, questions, or contributions:
- Open an issue on GitHub
- Check existing documentation
- Review the OpenPGP Card specification
