#!/bin/bash

PORT=${PORT:-3000}

/app/minichain-server --port $PORT --data-dir /app/data
