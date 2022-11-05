#!/bin/bash

# start peacock
cd Peacock

PORT=3000 ../node/bin/node ./chunk0.js &

# start game command
../run

