#!/usr/bin/python3

# a helper script to evaluate terminal sequences

# read in the sequence of numbers from the file "sequence.txt"
# will be in the format of a list of integers
# for example: 27, 93, 49, 49, 59, 63, 27, 92, 27, 91,
# 54, 110

# the sequence of numbers to decode

# grab the file name from the args. If no filename is specified use "sequence.bin"

import sys

filename = "sequence.bin"
# args
if len(sys.argv) > 1:
    filename = sys.argv[1]

# check that the file exists
try:
    with open(filename, "r") as f:
        # read the contents of the file
        contents = f.read()

except FileNotFoundError:
    print(f"File {filename} not found.")
    sys.exit(1)
except Exception as e:
    print(f"An error occurred: {e}")
    sys.exit(1)

# convert the contents to a list of integers
sequence = eval(contents)

# loop over the sequence and generate the decoded characters

decoded_characters = []
for number in sequence:
    decoded_characters.append(chr(number))

# join the decoded characters into a string
decoded_string = "".join(decoded_characters)

print(repr(decoded_string))  # Output the decoded string
