import sys
from pathlib import Path

sys.path.insert(0, "src")

collect_ignore = [path.name for path in Path(__file__).parent.glob("test_*.py")]
