import random
import sys

# get the number of lines to generate from args
if len(sys.argv) != 2:
    print("Usage: python random_crap.py <num_lines>")
    sys.exit(1)

num_lines = int(sys.argv[1])

# open a file

f = open(f"{num_lines}_lines.txt", "w")

for i in range(num_lines):
    line = "".join(
        [chr(random.randint(32, 126)) for i in range(random.randint(0, 100))]
    )
    f.write(line + "\n")

f.close()
