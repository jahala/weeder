#!/usr/bin/env bash
set -u
i=0
names=(tilth pleach tend2 copeca umbel)
paths=(/Users/jahala/CascadeProjects/tilth /Users/jahala/conductor/workspaces/pleach-v1/cayenne /Users/jahala/conductor/workspaces/feature-map/missoula /Users/jahala/conductor/workspaces/copeca/cancun /Users/jahala/conductor/workspaces/rctrl/master)
urls=(https://github.com/jahala/tilth.git https://github.com/jahala/pleach.git https://github.com/jahala/tend.git https://github.com/jahala/copeca.git https://github.com/jahala/umbel.git)
while [ $i -lt 5 ]; do
  n=${names[$i]}; p=${paths[$i]}; u=${urls[$i]}
  ref=$(git -C "$p" symbolic-ref --quiet refs/remotes/origin/HEAD)
  ls=$(git -C "$p" rev-parse "$ref")
  br=${ref#refs/remotes/origin/}
  rs=$(GIT_TERMINAL_PROMPT=0 git ls-remote "$u" "refs/heads/$br" | cut -f1)
  m=NO; [ "$ls" = "$rs" ] && m=yes
  echo "$n url=$u branch=$br local=$ls remote=$rs match=$m"
  i=$((i+1))
done
