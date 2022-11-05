#!/bin/bash

node_version="18.12.1"
node_url="https://nodejs.org/dist/v$node_version/node-v$node_version-linux-x64.tar.gz"

# Install node
wget -O node.tar.gz $node_url

tar -xzf node.tar.gz
rm node.tar.gz

mv node-v*-linux-x64/ node/