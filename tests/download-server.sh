#!/bin/bash
set -e
rm -rf colink-server
mkdir colink-server && cd colink-server
PACKAGE_NAME="colink-server-linux-x86_64.tar.gz"
if [ "$(uname)" == "Darwin" ]; then
    PACKAGE_NAME="colink-server-macos-x86_64.tar.gz"
fi
wget https://github.com/CoLearn-Dev/colink-server-dev/releases/download/v0.3.2/$PACKAGE_NAME
tar -xzf $PACKAGE_NAME
touch user_init_config.toml # create an empty user init config to prevent automatically starting protocols when importing users.
cd ..
