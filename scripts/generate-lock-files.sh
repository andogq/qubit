#!/bin/bash

MANIFESTS=$(find . -name Cargo.toml ! -path './target/*')

for MANIFEST in $MANIFESTS
do
    cargo generate-lockfile --manifest-path="$MANIFEST"
done
