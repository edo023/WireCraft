#!/usr/bin/env bash
# Prepara o aggiorna CADCraft (vendor/cadcraft) con i ganci di WireCraft.
#
#   scripts/aggiorna-cadcraft.sh            versione base (patches/cadcraft/BASE) + ganci: dopo un clone
#   scripts/aggiorna-cadcraft.sh --ultima   ultima versione di CADCraft + ganci; se i test passano,
#                                           la nuova versione diventa la base
#
# I ganci sono in patches/cadcraft/*.patch: piccoli e generici (comandi e menu di
# un'applicazione costruita su CADCraft), da proporre anche a CADCraft stesso.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
CC=vendor/cadcraft
BASE_FILE=patches/cadcraft/BASE

git submodule update --init "$CC"
if [[ "${1:-}" == "--ultima" ]]; then
    git -C "$CC" fetch origin
    BASE=$(git -C "$CC" rev-parse origin/main)
else
    BASE=$(cat "$BASE_FILE")
fi
echo "CADCraft base: $BASE"
git -C "$CC" checkout -q -B cadeo-hooks "$BASE"
git -C "$CC" -c user.name=cadeo -c user.email=cadeo@localhost am -q --3way "$ROOT"/patches/cadcraft/*.patch
echo "Ganci applicati"

# Compilazione fuori da cartelle sincronizzate (OneDrive): migliaia di file la rallentano.
if [[ -z "${CARGO_TARGET_DIR:-}" && -d /c/dev ]]; then
    export CARGO_TARGET_DIR=/c/dev/cadeo-target
fi
cargo test --release -p elettrico
if [[ "${1:-}" == "--ultima" ]]; then
    echo "$BASE" > "$BASE_FILE"
    echo "Nuova base registrata in $BASE_FILE"
fi
