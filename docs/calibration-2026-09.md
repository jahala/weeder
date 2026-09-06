# Calibration, 2026-09

weed is measured twice on the same code before it is allowed to block anyone: precision, how often it stops a commit that was fine, and recall, how often it catches an anti-pattern that is really there. A gate that fires on nothing has perfect precision, which is why neither number means anything without the other.

The precision half of this file is written by `cargo xtask calibrate` and is not in it yet. The recall half below is written by `cargo xtask mutate`.

<!-- recall:begin -->

## Recall

Every rule that blocks — T1, T3, S1, X1, C1, G1 — catches at least 95 percent of the anti-patterns planted for it, in each of ts, py, rs and go. 2608 cases were planted one anti-pattern at a time in real commits, and 2556 of them were caught on the site they were planted in.

The campaign is `cargo xtask mutate`. For each case it checks out a real commit of a corpus repository, plants one anti-pattern in its tree with a scanner that knows nothing about weed's detectors, and runs `weed check --base <parent> --strict`. A case counts as caught only where the rule fires on the file the shape was planted in, on the lines it was planted on where it has lines. A site the unmutated commit already fires that rule on is passed over, so no hit is inherited from the commit itself.

### The corpus

| Repository | Ref | Head | Commits walked | Cases |
|---|---|---|---|---|
| cobra | `adbc8813901bba65827259daa8e22ff94ec1f30e` | `adbc88139` | 200 | 300 |
| copeca | `origin/jahala/swebench-tasks` | `b121023e5` | 16 | 268 |
| hcl | `6abbb088cdb82416d1b3d9fcbaab29534133567a` | `6abbb088c` | 200 | 340 |
| pleach | `origin/master` | `49c9177c0` | 27 | 266 |
| tend2 | `origin/landing-rewrite` | `ea9a5004a` | 23 | 266 |
| tilth | `origin/main` | `08ae11a37` | 144 | 902 |
| umbel | `origin/master` | `b2e79e9d8` | 34 | 266 |

The garden five are the repositories calibration measures precision on. They carry no Go between them and weed judges Go, so the Go column is measured on two Go projects pinned by commit, read exactly the same way.

Files the injector planted nothing in, counted once for each commit they were read at:

- added by the commit, so nothing in it was there to change: 285
- carrying a NUL byte, which git writes no text diff for: 82
- saying a generator wrote it: 632

### Recall, per rule and language

| Rule | Level | Language | Cases | Hits | Misses | Recall |
|---|---|---|---|---|---|---|
| T1 | block | ts | 42 | 41 | 1 | 97.6% |
| T1 | block | py | 15 | 15 | 0 | 100.0% |
| T1 | block | rs | 40 | 40 | 0 | 100.0% |
| T1 | block | go | 40 | 40 | 0 | 100.0% |
| T2 | block | ts | 42 | 42 | 0 | 100.0% |
| T2 | block | py | 15 | 15 | 0 | 100.0% |
| T2 | block | rs | 40 | 40 | 0 | 100.0% |
| T2 | block | go | 40 | 40 | 0 | 100.0% |
| T3 | block | ts | 42 | 42 | 0 | 100.0% |
| T3 | block | py | 15 | 15 | 0 | 100.0% |
| T3 | block | rs | 40 | 40 | 0 | 100.0% |
| T3 | block | go | 40 | 40 | 0 | 100.0% |
| T4 | warn | ts | 42 | 39 | 3 | 92.9% |
| T4 | warn | py | 15 | 15 | 0 | 100.0% |
| T4 | warn | rs | 40 | 40 | 0 | 100.0% |
| T4 | warn | go | 0 | 0 | 0 | no cases |
| T5 | warn | ts | 42 | 42 | 0 | 100.0% |
| T5 | warn | py | 16 | 16 | 0 | 100.0% |
| T5 | warn | rs | 40 | 40 | 0 | 100.0% |
| T5 | warn | go | 20 | 20 | 0 | 100.0% |
| T6 | warn | ts | 42 | 28 | 14 | 66.7% |
| T6 | warn | py | 15 | 11 | 4 | 73.3% |
| T6 | warn | rs | 40 | 40 | 0 | 100.0% |
| T6 | warn | go | 20 | 20 | 0 | 100.0% |
| T7 | block | ts | 42 | 42 | 0 | 100.0% |
| T7 | block | py | 15 | 15 | 0 | 100.0% |
| T7 | block | rs | 0 | 0 | 0 | no cases |
| T7 | block | go | 40 | 40 | 0 | 100.0% |
| M1 | warn | ts | 42 | 42 | 0 | 100.0% |
| M1 | warn | py | 8 | 8 | 0 | 100.0% |
| M1 | warn | rs | 14 | 14 | 0 | 100.0% |
| M1 | warn | go | 40 | 40 | 0 | 100.0% |
| S1 | block | ts | 42 | 42 | 0 | 100.0% |
| S1 | block | py | 35 | 35 | 0 | 100.0% |
| S1 | block | rs | 40 | 40 | 0 | 100.0% |
| S1 | block | go | 40 | 40 | 0 | 100.0% |
| S2 | warn | ts | 42 | 42 | 0 | 100.0% |
| S2 | warn | py | 35 | 34 | 1 | 97.1% |
| S2 | warn | rs | 40 | 40 | 0 | 100.0% |
| S2 | warn | go | 40 | 40 | 0 | 100.0% |
| S3 | warn | ts | 42 | 42 | 0 | 100.0% |
| S3 | warn | py | 35 | 35 | 0 | 100.0% |
| S3 | warn | rs | 40 | 40 | 0 | 100.0% |
| S3 | warn | go | 40 | 40 | 0 | 100.0% |
| D1 | warn | ts | 42 | 42 | 0 | 100.0% |
| D1 | warn | py | 34 | 14 | 20 | 41.2% |
| D1 | warn | rs | 40 | 40 | 0 | 100.0% |
| D1 | warn | go | 20 | 20 | 0 | 100.0% |
| D2 | block | ts | 42 | 42 | 0 | 100.0% |
| D2 | block | py | 15 | 15 | 0 | 100.0% |
| D2 | block | rs | 40 | 40 | 0 | 100.0% |
| D2 | block | go | 20 | 11 | 9 | 55.0% |
| X1 | block | ts | 42 | 42 | 0 | 100.0% |
| X1 | block | py | 35 | 35 | 0 | 100.0% |
| X1 | block | rs | 40 | 40 | 0 | 100.0% |
| X1 | block | go | 40 | 40 | 0 | 100.0% |
| X2 | block | ts | 42 | 42 | 0 | 100.0% |
| X2 | block | py | 35 | 35 | 0 | 100.0% |
| X2 | block | rs | 40 | 40 | 0 | 100.0% |
| X2 | block | go | 40 | 40 | 0 | 100.0% |
| C1 | block | ts | 42 | 42 | 0 | 100.0% |
| C1 | block | py | 34 | 34 | 0 | 100.0% |
| C1 | block | rs | 40 | 40 | 0 | 100.0% |
| C1 | block | go | 40 | 40 | 0 | 100.0% |
| C2 | warn | ts | 42 | 42 | 0 | 100.0% |
| C2 | warn | py | 33 | 33 | 0 | 100.0% |
| C2 | warn | rs | 40 | 40 | 0 | 100.0% |
| C2 | warn | go | 40 | 40 | 0 | 100.0% |
| G1 | block | ts | 42 | 42 | 0 | 100.0% |
| G1 | block | py | 35 | 35 | 0 | 100.0% |
| G1 | block | rs | 40 | 40 | 0 | 100.0% |
| G1 | block | go | 40 | 40 | 0 | 100.0% |
| G2 | warn | ts | 42 | 42 | 0 | 100.0% |
| G2 | warn | py | 36 | 36 | 0 | 100.0% |
| G2 | warn | rs | 40 | 40 | 0 | 100.0% |
| G2 | warn | go | 40 | 40 | 0 | 100.0% |

### Every miss

Each of these was planted and not reported. The before and after of every one is kept under the campaign's cache directory, as `cases/<rule>/<language>/<repository>-<commit>`, so a number nobody believes can be replayed by hand. `WEED_RECALL_CACHE` says where that directory is; it sits under the temporary directory otherwise.

- T6 · py · copeca `b121023e5` · `tests/config/test_loader.py:37` — an error assertion stopped naming the error
- T6 · py · copeca `1b01df97f` · `tests/runners/test_base_runner.py:62` — an error assertion stopped naming the error
- S2 · py · copeca `387932ad5` · `tests/e2e/fake_agent.py:61` — a handler was added that catches and says nothing
- T6 · py · copeca `41cfc63ae` · `tests/config/test_loader.py:37` — an error assertion stopped naming the error
- T6 · py · copeca `70d669542` · `tests/config/test_scenario_loader.py:38` — an error assertion stopped naming the error
- D2 · go · hcl `6bf1a67a9` · `gohcl/decode.go:6` — `gohcl` was made to import `hcldec`
- D2 · go · hcl `9466647a1` · `hclwrite/ast_block.go:6` — `hclwrite` was made to import `integrationtest`
- D2 · go · hcl `ab1acc486` · `hclwrite/tokens.go:6` — `hclwrite` was made to import `integrationtest`
- D2 · go · hcl `2efc26623` · `hclwrite/ast_body.go:6` — `hclwrite` was made to import `integrationtest`
- D2 · go · hcl `bd45ab812` · `hclwrite/format.go:6` — `hclwrite` was made to import `integrationtest`
- D2 · go · hcl `6a91a7547` · `gohcl/types.go:6` — `gohcl` was made to import `hcldec`
- D2 · go · hcl `e73f21667` · `gohcl/schema.go:6` — `gohcl` was made to import `hcldec`
- D2 · go · hcl `92f12c4e5` · `hcldec/gob.go:6` — `hcldec` was made to import `hcled`
- D2 · go · hcl `0268c1604` · `gohcl/types.go:6` — `gohcl` was made to import `hcldec`
- T4 · ts · pleach `3a1306011` · `test/loop/run-work.test.ts:10` — a wait was widened from 1000 to 10000
- T6 · ts · tend2 `ea9a5004a` · `test/route.test.ts:143` — an error assertion stopped naming the error
- T6 · ts · tend2 `4c89e6b72` · `test/route.test.ts:143` — an error assertion stopped naming the error
- T6 · ts · tend2 `51035a5c8` · `test/verify.test.ts:176` — an error assertion stopped naming the error
- T6 · ts · tend2 `6d9a1cf91` · `test/route.test.ts:143` — an error assertion stopped naming the error
- T6 · ts · tend2 `044912212` · `test/route.test.ts:143` — an error assertion stopped naming the error
- T6 · ts · tend2 `5b1f7a471` · `test/verify.test.ts:176` — an error assertion stopped naming the error
- T6 · ts · tend2 `32df8ddd1` · `test/verify.test.ts:176` — an error assertion stopped naming the error
- T1 · ts · tend2 `8d939ec15` · `test/renderer-fresh.test.ts` — the case `it` was deleted
- T6 · ts · tend2 `8d939ec15` · `test/route.test.ts:143` — an error assertion stopped naming the error
- T6 · ts · tend2 `42c840a73` · `test/verify.test.ts:176` — an error assertion stopped naming the error
- T6 · ts · tend2 `ad070a0d4` · `test/verify.test.ts:176` — an error assertion stopped naming the error
- T6 · ts · tend2 `2a0e93933` · `test/verify.test.ts:176` — an error assertion stopped naming the error
- T6 · ts · tend2 `e2adbab97` · `test/verify.test.ts:176` — an error assertion stopped naming the error
- T6 · ts · tend2 `e9e8387e9` · `test/verify.test.ts:176` — an error assertion stopped naming the error
- T6 · ts · tend2 `99296a937` · `test/verify.test.ts:176` — an error assertion stopped naming the error
- D1 · py · tilth `08ae11a37` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `bd7444b42` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `c769da5dd` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `3d0968130` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `2c8280bd0` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `bb7900b8c` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `1dfc5d26b` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `5287e9fce` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `30fd445b0` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `1c70eeeaa` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `f35864f33` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `69a20b97c` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `f6e9fbe52` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `589054c54` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `a9b648e0b` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `79503f3df` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `f8f701ab5` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `c2334ee01` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `f761368c9` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- D1 · py · tilth `cc64a4259` · `benchmark/fixtures/setup.py` — a dependency pin was moved
- T4 · ts · umbel `ea995c590` · `test/unit/errors.test.ts:61` — a wait was widened from 5000 to 50000
- T4 · ts · umbel `e4f19b13d` · `test/unit/errors.test.ts:61` — a wait was widened from 5000 to 50000

### Where a shape had nowhere to go

- T4 · go — no commit of the corpus held a site for this shape: 400 commits offered none, and 0 more were passed over because the commit itself already fires the rule there
- T7 · rs — the language's runner does not collect by name, so no rename takes a case out of the run

<!-- recall:end -->
