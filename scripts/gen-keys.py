#!/usr/bin/env -S uv run --quiet
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "cryptography",
# ]
# ///
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
key = Ed25519PrivateKey.generate()
pub = key.public_key().public_bytes_raw().hex()
priv = key.private_bytes_raw().hex()
print(f"TEST_PUBLIC_KEY={pub}\nTEST_PRIVATE_KEY={priv}")
