#!/bin/bash -eu
# Copyright 2026 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
################################################################################

cd $SRC/rullst

# 1. Build rullst-security fuzz targets
cd rullst-security
cargo fuzz build -O --debug-assertions
for target in fuzz/fuzz_targets/*.rs; do
    target_name=$(basename "$target" .rs)
    if [ -f "target/x86_64-unknown-linux-gnu/release/$target_name" ]; then
        cp target/x86_64-unknown-linux-gnu/release/$target_name $OUT/
    fi
done
cd ..

# 2. Build rullst-core / workspace fuzz targets
if [ -d "fuzz" ]; then
    cargo fuzz build -O --debug-assertions
    for target in fuzz/fuzz_targets/*.rs; do
        target_name=$(basename "$target" .rs)
        if [ -f "target/x86_64-unknown-linux-gnu/release/$target_name" ]; then
            cp target/x86_64-unknown-linux-gnu/release/$target_name $OUT/
        fi
    done
fi

# 3. Build rullst-orm fuzz targets
if [ -d "rullst-orm/fuzz" ]; then
    cd rullst-orm
    cargo fuzz build -O --debug-assertions
    for target in fuzz/fuzz_targets/*.rs; do
        target_name=$(basename "$target" .rs)
        if [ -f "target/x86_64-unknown-linux-gnu/release/$target_name" ]; then
            cp target/x86_64-unknown-linux-gnu/release/$target_name $OUT/
        fi
    done
    cd ..
fi

# 4. Build rullst-connect fuzz targets
if [ -d "rullst-connect/fuzz" ]; then
    cd rullst-connect
    cargo fuzz build -O --debug-assertions
    for target in fuzz/fuzz_targets/*.rs; do
        target_name=$(basename "$target" .rs)
        if [ -f "target/x86_64-unknown-linux-gnu/release/$target_name" ]; then
            cp target/x86_64-unknown-linux-gnu/release/$target_name $OUT/
        fi
    done
    cd ..
fi
