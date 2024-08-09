#!/usr/bin/env bash

set -e
set -o pipefail

# projectPath=/c/Projects/eris/liquid-staking-contracts
projectPath=$(dirname `pwd`) 

docker run --security-opt seccomp=unconfined -v "/$projectPath":/volume \
  --mount type=bind,source=/$projectPath-cache/registry,target=/usr/local/cargo/registry \
  xd009642/tarpaulin