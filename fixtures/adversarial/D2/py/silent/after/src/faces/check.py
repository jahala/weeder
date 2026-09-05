from src.core.finding import render
from src.seams.git import read


def check(path):
    return render("T1", read(path))
