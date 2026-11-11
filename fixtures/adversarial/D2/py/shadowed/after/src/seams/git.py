import subprocess


def read(path):
    return subprocess.run(["git", "show", path], capture_output=True).stdout
