#!/usr/bin/env -S uv run --quiet
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "cryptography",
# ]
# ///
"""
Send a signed Discord interaction payload to a server.

Usage:
    ./scripts/send-message.py -d '<json>' <host>

Example:
    ./scripts/send-message.py -d '{"type": 1}' localhost:8080

The cryptographic keys are read from the TEST_PUBLIC_KEY and TEST_PRIVATE_KEY
environment variables, or from a .env file in the current directory.

Note that the server must also be configured to use the same public key,
i.e.:
    cargo run -- --public-key "$TEST_PUBLIC_KEY"
"""

import argparse
import json
import os
import random
import subprocess
import sys
import time
from pathlib import Path


def load_key(key_name: str) -> str:
    """Return the named key from the environment or .env file."""
    if key := os.environ.get(key_name):
        return key

    env_file = Path(".env")
    if env_file.exists():
        for line in env_file.read_text().splitlines():
            line = line.strip()
            if line.startswith("#") or "=" not in line:
                continue
            k, _, v = line.partition("=")
            if k.strip() == key_name:
                return v.strip().strip('"').strip("'")

    print("error: PUBLIC_KEY not set in environment or .env file", file=sys.stderr)
    sys.exit(1)

def load_public_key() -> str:
    """Return PUBLIC_KEY from the environment or .env file."""
    return load_key("TEST_PUBLIC_KEY")

def load_private_key() -> str:
    """Return PRIVATE_KEY from the environment or .env file."""
    return load_key("TEST_PRIVATE_KEY")


def sign(public_key_hex: str, timestamp: str, body: str) -> str:
    """Return a hex Ed25519 signature over (timestamp + body)."""
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

    # Discord's public key is the *verify* key; for local testing we need
    # the private key that corresponds to it.  The PUBLIC_KEY env var holds
    # the 32-byte public key in hex — but to *produce* a valid signature we
    # need the private key.
    #
    # Convention adopted here: if you set PRIVATE_KEY (hex) in your env or
    # .env, we use that.  Otherwise we derive a throw-away key from the public
    # key bytes so the request reaches your handler (which will then reject it
    # if signature verification is strict).  For real testing, set PRIVATE_KEY
    # to the 32-byte seed that corresponds to your PUBLIC_KEY.
    private_key_hex = load_private_key()

    private_key = Ed25519PrivateKey.from_private_bytes(
        bytes.fromhex(private_key_hex)
    )

    message = (timestamp + body).encode()
    signature_bytes = private_key.sign(message)
    return signature_bytes.hex()

def random_snowflake() -> str:
    """Generate a plausible random Discord snowflake (64-bit integer as string)."""
    # Discord snowflakes encode a timestamp in the upper bits; for testing,
    # any large integer in the right range is fine.
    return str(random.randint(10 ** 17, 10 ** 18 - 1))


def build_interaction(args: argparse.Namespace) -> str:
    """Construct the interaction JSON body from args, filling in random values as needed."""
    payload = {
        "id":                            args.id or random_snowflake(),
        "application_id":                args.application_id or random_snowflake(),
        "type":                          args.type,
        "token":                         args.token or "test-token",
        "version":                       1,
        "entitlements":                  [],
        "authorizing_integration_owners": {},
    }

    # type 2 = APPLICATION_COMMAND; include a minimal data block unless type is PING (1)
    if args.type != 1:
        payload["data"] = {
            "id":   args.data_id or random_snowflake(),
            "name": args.data_name or "ping",
            "type": 1,  # CHAT_INPUT
        }

    return json.dumps(payload, separators=(',',':'))


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Send a signed Discord interaction payload via curl."
    )

    # Transport
    parser.add_argument("-d", "--data", metavar="JSON",
                        help="Raw JSON body (overrides all --field arguments)")
    parser.add_argument("host", help='Target host, e.g. "localhost:8080"')

    # Interaction fields (all optional; random/default values used if absent)
    parser.add_argument("--id",             metavar="SNOWFLAKE", help="interaction id")
    parser.add_argument("--application-id", metavar="SNOWFLAKE", help="application_id")
    parser.add_argument("--type",           metavar="INT", type=int, default=2,
                        help="interaction type (default: 2 = APPLICATION_COMMAND; use 1 for PING)")
    parser.add_argument("--token",          metavar="STR",       help="interaction token")
    parser.add_argument("--data-id",        metavar="SNOWFLAKE", help="data.id (command id)")
    parser.add_argument("--data-name",      metavar="STR",       help="data.name (command name)")

    args = parser.parse_args()

    body = args.data if args.data else build_interaction(args)

    public_key = load_public_key()
    timestamp  = str(int(time.time()))
    signature  = sign(public_key, timestamp, body)

    url = f"http://{args.host}/"
    cmd = [
        "curl", "-s", "-i",
        "-X", "POST",
        "-H", "Content-Type: application/json",
        "-H", f"X-Signature-Ed25519: {signature}",
        "-H", f"X-Signature-Timestamp: {timestamp}",
        "-d", body,
        url,
    ]

    print(f"POST {url}", file=sys.stderr)
    print(f"  X-Signature-Timestamp:   {timestamp}", file=sys.stderr)
    print(f"  X-Signature-Ed25519:     {signature}", file=sys.stderr)
    print(f"  body:                    {body}", file=sys.stderr)
    print(file=sys.stderr)

    result = subprocess.run(cmd)
    print()
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
