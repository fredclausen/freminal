import random

# we want 1000 lines of random ASCII text. Line length should vary between 1 and 100 characters.

# open a file

f = open("random_crap.txt", "w")

for i in range(10000):
    line = "".join(
        [chr(random.randint(32, 126)) for i in range(random.randint(0, 100))]
    )
    f.write(line + "\n")

f.close()
