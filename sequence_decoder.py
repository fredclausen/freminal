#!/usr/bin/python3

# a helper script to evaluate terminal sequences

# read in the sequence of numbers from the file "sequence.txt"
# will be in the format of a list of integers
# for example: 27, 93, 49, 49, 59, 63, 27, 92, 27, 91,
# 54, 110

# the sequence of numbers to decode

# grab the file name from the args. If no filename is specified use "sequence.bin"

# possible args are --recording-path <path> and --convert-escape

import sys

filename = "sequence.bin"
convert_escape = False
split_commands = False

# loop over the args

for arg in sys.argv[1:]:
    if arg.startswith("--recording-path"):
        filename = arg.split("=")[1]
    elif arg == "--convert-escape":
        convert_escape = True
    elif arg == "--split-commands":
        split_commands = True

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

if convert_escape:
    # find all of the \x1b sequences and replace with ESC
    decoded_string = decoded_string.replace("\x1b", "ESC")

if split_commands:
    # split the string into commands based on the ESC character
    commands = decoded_string.split("ESC")
    i = 0
    for command in commands:
        print(f"N{i} ESC " + repr(command))  # Output each command
        i += 1
else:
    print(repr(decoded_string))  # Output the decoded string
