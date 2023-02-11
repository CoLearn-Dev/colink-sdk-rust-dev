#!/bin/bash
set -e
rm -rf colink-server
mkdir colink-server && cd colink-server
wget https://github.com/CoLearn-Dev/colink-server-dev/releases/download/v0.3.2/colink-server-linux-x86_64.tar.gz
tar -xzf colink-server-linux-x86_64.tar.gz
touch user_init_config.toml # create an empty user init config to prevent automatically starting protocols when importing users.
cd ..
