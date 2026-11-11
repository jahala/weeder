import sys

from src.seams.git import read


def main():
    print(read(sys.argv[1]))
