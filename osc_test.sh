#!/usr/bin/env bash
read -rs -d \\ -p $'\e]11;?\e\\' BG

echo "Formatting one: "
echo "$BG" |
  xxd -c 64 #|
# grep -o -E "rgb:.{4}/.{4}/.{4}"

echo "Formatting two: "

echo "$BG" |
  xxd -c 64 |
  grep -o -E "rgb:.{4}/.{4}/.{4}"
