#!/usr/bin/env python3
value = []

# the buffer should represent a utf8 string. Convert the whole buffer and see if it is a valid utf8 string
while True:
    try:
        print(bytes(value).decode("utf-8"))
        break
    except UnicodeDecodeError:
        print("Invalid utf8 string")
        # pop a byte from the end of the buffer
        value.pop()
