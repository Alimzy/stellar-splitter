#!/usr/bin/env bash
# Testnet lifecycle: deploy -> create_split (70/30) -> deposit -> claim x2.
#
# Usage: scripts/demo-testnet.sh [--dry-run] [--help]
#
# Needs stellar-cli 28.x and three funded testnet identities. Create them with
#   stellar keys generate --fund alice --network testnet   (also bob, carol)
# This script never creates, prints, or stores secret keys.
set -euo pipefail

DRY=0
case "${1:-}" in
  --help|-h) sed -n '2,9p' "$0"; exit 0 ;;
  --dry-run) DRY=1 ;;
  "") ;;
  *) echo "unknown argument: $1" >&2; exit 2 ;;
esac

NETWORK=testnet
WASM=target/wasm32v1-none/release/stellar_splitter.wasm
AMOUNT=10000000   # 1 XLM in stroops

run() {
  echo "+ $*" >&2
  if [ "$DRY" -eq 0 ]; then "$@"; fi
}

if [ "$DRY" -eq 1 ]; then
  ALICE=ALICE_ADDRESS; BOB=BOB_ADDRESS; CAROL=CAROL_ADDRESS
  TOKEN=NATIVE_ASSET_CONTRACT_ID; ID=SPLITTER_CONTRACT_ID
else
  for k in alice bob carol; do
    stellar keys address "$k" >/dev/null 2>&1 \
      || { echo "missing identity '$k' (see header of this script)" >&2; exit 1; }
  done
  ALICE=$(stellar keys address alice)
  BOB=$(stellar keys address bob)
  CAROL=$(stellar keys address carol)
  TOKEN=$(stellar contract id asset --asset native --network "$NETWORK")
fi

run stellar contract build --locked --profile release
if [ "$DRY" -eq 0 ]; then
  echo "wasm sha256: $(sha256sum "$WASM" | cut -d' ' -f1)"
  ID=$(stellar contract deploy --wasm "$WASM" --source alice --network "$NETWORK")
  echo "splitter contract: $ID"
else
  run stellar contract deploy --wasm "$WASM" --source alice --network "$NETWORK"
fi

RECIPIENTS="[{\"address\":\"$BOB\",\"bps\":7000},{\"address\":\"$CAROL\",\"bps\":3000}]"
inv() { local src=$1; shift; run stellar contract invoke --id "$ID" --source "$src" --network "$NETWORK" -- "$@"; }

inv alice create_split --creator "$ALICE" --token "$TOKEN" --recipients "$RECIPIENTS"
inv alice deposit --split_id 1 --from "$ALICE" --amount "$AMOUNT"
inv bob   claim   --split_id 1 --recipient "$BOB"
inv carol claim   --split_id 1 --recipient "$CAROL"

echo "expected: bob +7000000, carol +3000000 stroops, contract balance 0"
