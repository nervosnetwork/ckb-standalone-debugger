# Folder text reports

import sys

hmap = {}

for line in sys.stdin:
    line = line.rstrip()
    prefix, cycles = line.rsplit(' ', 1)
    cycles = int(cycles)
    if prefix not in hmap:
        hmap[prefix] = 0
    hmap[prefix] += cycles

for k, v in hmap.items():
    print(k, v)
