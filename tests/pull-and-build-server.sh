#!/bin/bash
set -e
rm -rf colink-server
git clone --recursive git@github.com:CoLearn-Dev/colink-server-dev.git -b v0.1.7 colink-server
cd colink-server
cargo build
cd ..
