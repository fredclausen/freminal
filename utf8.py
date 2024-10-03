#!/usr/bin/python3

import codecs

sequence = [226, 157, 175, 32]
sequence_as_bytes = bytes(sequence)

# validate sequence is utf-8

strData = codecs.decode(sequence_as_bytes, "UTF-8")

print(f"yo {strData}")
