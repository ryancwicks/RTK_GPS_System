#!/bin/bash
cd ..
cargo run --release -- --start rtk-rover --mount-point ${NTRIP_MOUNT_POINT} --username ${NTRIP_USERNAME} --server ${NTRIP_SERVER}
