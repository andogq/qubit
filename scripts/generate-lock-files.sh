#!/bin/bash

MANIFESTS=$(find . -name Cargo.toml ! -path './target/*')

for MANIFEST in $MANIFESTS
do
    echo "Updating Cargo.lock for $MANIFEST"

    cargo generate-lockfile --manifest-path="$MANIFEST"
done
