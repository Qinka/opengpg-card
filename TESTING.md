# Testing Guide

## Overview

This document describes how to test the ESP32-S3 OpenPGP Card implementation.

## Unit Tests

### Running Tests

Most of the codebase is `no_std` embedded code, which limits traditional unit testing. However, some modules include unit tests that can run on the host:

```bash
# Run tests for modules with #[cfg(test)]
cargo test --lib

# Run specific module tests
cargo test --lib pin::tests
cargo test --lib apdu::tests
cargo test --lib crypto::tests
```

### Test Coverage

Current test coverage:
- `src/pin.rs`: Basic PIN verification logic
- `src/apdu.rs`: APDU parsing
- `src/crypto.rs`: Key generation and hashing

## Hardware Testing

### Prerequisites

1. ESP32-S3 development board
2. USB cable (data-capable)
3. Computer with GnuPG installed

### Flashing the Firmware

```bash
# Build and flash
cargo run --release

# Or manually
espflash flash target/xtensa-esp32s3-none-elf/release/opengpg-card
```

### Serial Monitor Tests

1. **Basic Initialization Test**
   ```bash
   espflash monitor
   ```
   
   Expected output:
   ```
   ESP32-S3 OpenPGP Card Initializing...
   Hardware initialized
   Storage initialized
   OpenPGP card state initialized
   ESP32-S3 OpenPGP Card Ready!
   Waiting for USB CCID commands...
   ```

2. **Periodic Heartbeat**
   
   Every minute, you should see:
   ```
   Card alive - waiting for commands (1m)
   Card alive - waiting for commands (2m)
   ...
   ```

## Integration Tests with GnuPG

### Test 1: Card Detection

```bash
gpg --card-status
```

Expected output:
```
Reader ...........: ESP32-S3 OpenPGP Card
Application ID ...: D27600012401...
Version ..........: 3.4
```

If this fails:
- Check USB connection
- Restart pcscd: `sudo systemctl restart pcscd`
- Check device permissions
- Verify firmware is running (check serial monitor)

### Test 2: PIN Verification

```bash
gpg --card-edit
> verify
```

Enter default User PIN: `123456`

Expected: Success message

Test incorrect PIN:
```
> verify
```
Enter wrong PIN: `000000`

Expected: Error message, retry counter decremented

### Test 3: Admin PIN Verification

```bash
gpg --card-edit
> admin
> verify
```

Enter default Admin PIN: `12345678`

Expected: Success message

### Test 4: Change User PIN

```bash
gpg --card-edit
> passwd
> 1
```

Follow prompts to change User PIN.

Expected: Success message

Verify new PIN works:
```bash
gpg --card-edit
> verify
```

### Test 5: Key Generation

```bash
gpg --card-edit
> admin
> generate
```

Follow prompts to generate keys on card.

Expected:
- Keys generated successfully
- Public key exported
- Keys stored in keyring

### Test 6: Signing Test

```bash
# Create a test file
echo "Test message" > test.txt

# Sign with card
gpg --sign test.txt
```

Enter User PIN when prompted.

Expected:
- Signature created: `test.txt.gpg`

Verify signature:
```bash
gpg --verify test.txt.gpg
```

Expected: Good signature

### Test 7: Decryption Test

```bash
# Encrypt a file
echo "Secret message" > secret.txt
gpg --encrypt --recipient YOUR_EMAIL secret.txt

# Decrypt with card
gpg --decrypt secret.txt.gpg
```

Enter User PIN when prompted.

Expected: Decrypted message displayed

### Test 8: Authentication Test (SSH)

```bash
# Export SSH public key
gpg --export-ssh-key YOUR_EMAIL > ~/.ssh/id_openpgp.pub

# Test SSH connection with card
ssh-add -L
```

Expected: Public key listed

### Test 9: PIN Blocking Test

**Warning:** This will block your PIN. Have Admin PIN ready to reset.

```bash
gpg --card-edit
> verify
```

Enter incorrect PIN 3 times.

Expected: PIN blocked, cannot verify

Reset with Admin PIN:
```bash
gpg --card-edit
> admin
> unblock
```

Expected: User PIN reset

## Performance Tests

### Test 1: Key Generation Performance

```bash
time gpg --card-edit --command-fd 0 <<EOF
admin
generate
y
0
Test User
test@example.com
EOF
```

Measure time for key generation.

### Test 2: Signing Performance

```bash
# Generate 100 test files
for i in {1..100}; do echo "Test $i" > test_$i.txt; done

# Time signing all files
time for i in {1..100}; do 
  gpg --pinentry-mode loopback --passphrase "123456" --sign test_$i.txt
done
```

### Test 3: Throughput Test

```bash
# Create 1MB test file
dd if=/dev/urandom of=large.bin bs=1M count=1

# Time encryption
time gpg --encrypt --recipient YOUR_EMAIL large.bin

# Time decryption
time gpg --decrypt large.bin.gpg > /dev/null
```

## Stress Tests

### Test 1: Rapid PIN Verification

Test PIN verification under rapid requests:

```bash
for i in {1..100}; do
  gpg --card-edit --command-fd 0 <<EOF
verify
123456
quit
EOF
done
```

Expected: All verifications succeed, no crashes

### Test 2: Long-Running Operations

Keep card active for extended period:

```bash
# Sign files repeatedly for 1 hour
timeout 3600 bash -c 'while true; do 
  echo "Test" | gpg --sign > /dev/null 2>&1
  sleep 1
done'
```

Expected: No crashes, memory leaks, or errors

### Test 3: USB Reconnection Test

```bash
# In a loop: disconnect and reconnect USB
for i in {1..10}; do
  echo "Disconnect USB cable now"
  sleep 5
  echo "Reconnect USB cable now"
  sleep 5
  gpg --card-status
done
```

Expected: Card recovers after each reconnection

## Security Tests

### Test 1: Timing Attack Resistance

Use specialized tools to measure PIN verification timing:

```bash
# Install timing attack tools (example)
# pip install timing-attack-analyzer

# Perform timing analysis
python timing_test.py --target "PIN verification"
```

Expected: Constant time regardless of PIN correctness

### Test 2: Side-Channel Analysis

Requires specialized equipment:
- Oscilloscope
- Power analysis tools
- EM probes

Monitor power consumption during:
- PIN verification
- Key generation
- Signing operations

### Test 3: Fault Injection

Test robustness against:
- Voltage glitching
- Clock glitching
- Temperature variations

## Automated Testing

### CI/CD Testing

Create automated tests for CI/CD:

```bash
#!/bin/bash
# tests/integration_test.sh

set -e

echo "Testing card detection..."
gpg --card-status > /dev/null

echo "Testing PIN verification..."
gpg --card-edit --command-fd 0 <<EOF
verify
123456
quit
EOF

echo "Testing key generation..."
# ... more tests

echo "All tests passed!"
```

### Continuous Monitoring

Set up monitoring for:
- Memory usage
- Error rates
- Response times
- Crash reports

## Test Results Template

Document test results:

```markdown
## Test Run: 2024-XX-XX

### Environment
- Board: ESP32-S3-DevKitC-1
- Firmware: v0.1.0
- GnuPG: 2.x.x
- OS: Ubuntu 22.04

### Results
| Test | Status | Notes |
|------|--------|-------|
| Card Detection | ✅ PASS | - |
| PIN Verification | ✅ PASS | - |
| Key Generation | ✅ PASS | Took 5.2s |
| Signing | ✅ PASS | - |
| Decryption | ✅ PASS | - |
| PIN Blocking | ✅ PASS | - |

### Issues Found
1. None

### Recommendations
1. ...
```

## Troubleshooting Test Failures

### Card Not Detected

1. Check USB connection
2. Verify firmware is running (`espflash monitor`)
3. Check `dmesg` for USB errors
4. Try different USB port/cable

### PIN Verification Fails

1. Verify using default PIN
2. Check serial logs for errors
3. Reset card to defaults
4. Reflash firmware

### Key Generation Fails

1. Check available memory
2. Monitor serial output for errors
3. Verify RNG is working
4. Check for timeout issues

### Performance Issues

1. Check CPU frequency (should be 240MHz)
2. Verify optimization level (`--release`)
3. Monitor for memory allocation failures
4. Check for infinite loops in serial output

## Contributing Tests

When adding new features, include:
1. Unit tests (where applicable)
2. Integration test procedures
3. Expected results
4. Edge cases to test

## Test Coverage Goals

- [ ] 80% code coverage for testable modules
- [ ] All OpenPGP commands tested
- [ ] All error conditions tested
- [ ] Performance benchmarks established
- [ ] Security tests documented
- [ ] Regression tests for bug fixes
