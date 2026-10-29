#!/usr/bin/env bash
# Runs docker compose with the chosen servers on their dev stack (live reload), the rest on
# the local production stack. Name the servers, or run without any to pick from a menu.
#
#   ./dev.sh                                  menu, then up --watch (up --build if none chosen)
#   ./dev.sh auth-server spec-provider        those two in dev, then up --watch
#   ./dev.sh all                              every server in dev
#   ./dev.sh auth-server -- down              anything after -- replaces the default command
#
# Use the same servers for down and logs as for up.
set -euo pipefail
cd "$(dirname "$0")"

if [ -z "${MKCERT_CAROOT:-}" ] && command -v mkcert >/dev/null; then
  export MKCERT_CAROOT="$(mkcert -CAROOT)"   # the servers mount its rootCA.pem
fi

SERVERS=(identity-provider auth-server)

chosen=()
while [ $# -gt 0 ] && [ "$1" != "--" ]; do
  chosen+=("$1")
  shift
done
[ "${1:-}" = "--" ] && shift
custom=$#

if [ ${#chosen[@]} -eq 0 ]; then
  echo "Servers to run in dev (numbers separated by spaces, 'all', or empty for none):"
  for i in "${!SERVERS[@]}"; do echo "  $((i + 1))) ${SERVERS[$i]}"; done
  read -r -p "> " answer
  for item in $answer; do
    case "$item" in
      all) chosen+=(all) ;;
      [1-4]) chosen+=("${SERVERS[$((item - 1))]}") ;;
      *) echo "Unknown choice: $item" >&2; exit 1 ;;
    esac
  done
fi

for server in ${chosen[@]+"${chosen[@]}"}; do
  if [ "$server" = "all" ]; then
    for s in "${SERVERS[@]}"; do
      name="DEV_$(echo "$s" | tr 'a-z-' 'A-Z_')"
      export "$name=../$s/docker-compose.dev.yml"
    done
    continue
  fi
  case " ${SERVERS[*]} " in
    *" $server "*) ;;
    *) echo "Unknown server: $server (one of: ${SERVERS[*]}, all)" >&2; exit 1 ;;
  esac
  name="DEV_$(echo "$server" | tr 'a-z-' 'A-Z_')"
  export "$name=../$server/docker-compose.dev.yml"
done

# Watch only when a server is in dev: there is nothing to watch on the production stack.
if [ "$custom" -eq 0 ]; then
  if [ ${#chosen[@]} -gt 0 ]; then set -- up --watch; else set -- up --build; fi
fi

echo "Dev: $(env | grep '^DEV_' | sed 's/=.*//' | tr '\n' ' ')"
exec docker compose "$@"
