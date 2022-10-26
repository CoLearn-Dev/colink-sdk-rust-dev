#!/bin/bash
set -e
rm -rf colink-server
mkdir colink-server && cd colink-server
wget https://github.com/CoLearn-Dev/colink-server-dev/releases/download/v0.2.0/colink-server-linux-x86_64.tar.gz
tar -xzf colink-server-linux-x86_64.tar.gz
touch user_init_config.toml
cd ..
