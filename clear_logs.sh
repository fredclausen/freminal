#!/bin/sh

# Clear log *.log

# get a list of files we'll remove
# we want to remove ./*.log

echo "Removing"
ls -l ./*.log
rm -f ./*.log
