#!/bin/bash
cd ..
cargo run --release -- --start rtk-base --mount-point ${NTRIP_MOUNT_POINT} --username ${NTRIP_USERNAME} --server ${NTRIP_SERVER} --password ${NTRIP_PASSWORD} # --fixed_ECEF_x ${ECEF_X} --fixed_ECEF_Y ${ECEF_Y} --fixed_ECEF_Z ${ECEF_Z} --fixed_ECEF_accuracy ${ECEF_ACC}