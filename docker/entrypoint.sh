#!/bin/sh
set -eu

app_dir=/app
config_path="${CONFIG_PATH:-$app_dir/config.toml}"

if [ ! -e "$config_path" ]; then
  cp /usr/local/share/bamboolog/config.toml "$config_path"
fi

chown -R bamboolog:bamboolog "$app_dir"

exec su-exec bamboolog:bamboolog bamboolog "$@"
