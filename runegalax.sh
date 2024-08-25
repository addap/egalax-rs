#!/bin/sh

set -xe

sudo -i -u pi bash << EOF

cd ./egalax-rs/
export DISPLAY=:0
export RUST_LOG=info
/home/pi/.cargo/bin/egalax-rs /dev/egalax

EOF


