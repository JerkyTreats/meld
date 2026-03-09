# Changelog

## [2.1.0](https://github.com/JerkyTreats/meld/compare/v2.0.0...v2.1.0) — 2026-03-08

### Features

* **cli:** add live context generation feedback [2a501ee](https://github.com/JerkyTreats/meld/commit/2a501eef0a75bc0cd19b5fb4f1fdbc543e59c7a3)

### Bug Fixes

* **workflow:** improve docs writer readme synthesis [d99ab36](https://github.com/JerkyTreats/meld/commit/d99ab36c7543fd543af0e30d98c626f2d5ac0318)

### Design

* **workflow:** add publish arbiter workflow spec [50b4a85](https://github.com/JerkyTreats/meld/commit/50b4a85063e34da6359fffb5bb0d185550a5024c)


## [2.0.0](https://github.com/JerkyTreats/meld/compare/v1.2.0...v2.0.0) — 2026-03-07

### ⚠ BREAKING CHANGES

* fallback runtime state paths no longer write under <workspace>/.meld and now resolve to external data roots.

### Features

* **workflow:** add profile schema and layered registry loader [0b41666](https://github.com/JerkyTreats/meld/commit/0b41666819800729682abe3d80a91b200dc61647)
* **agent:** add workflow binding and registry validation [8b12782](https://github.com/JerkyTreats/meld/commit/8b127821f04f42f96b5439c0e1e5662c28cbb24a)
* **workflow:** execute bound turn workflows in context generate [9932cdc](https://github.com/JerkyTreats/meld/commit/9932cdc2b0fef333f5a3aeafd876cef29dbbf008)
* **workflow:** persist thread turn state and resume runs [a17ddef](https://github.com/JerkyTreats/meld/commit/a17ddeff93a8d2fa2d0eb86d90430931b5f611e6)
* **workflow:** resolve artifact prompt refs via verified storage [879cebe](https://github.com/JerkyTreats/meld/commit/879cebe42a591fa3c10ab7cbd5dab69ff47f5ca4)
* **workflow:** add workflow CLI surface and watch bound routing [61ae111](https://github.com/JerkyTreats/meld/commit/61ae1112947ee5d35709e3b35324caa3b8f0007b)
* **workflow:** emit telemetry hooks across workflow execution paths [3cecfc9](https://github.com/JerkyTreats/meld/commit/3cecfc98361a6aa06466c403341a2ce8afc96989)
* **telemetry:** report workflow progress in context generation [94d12b3](https://github.com/JerkyTreats/meld/commit/94d12b375ac13b53b040d7420c2611f4cffb1750)

### Bug Fixes

* **storage:** persist workflow and head index state outside workspace [46cd5f1](https://github.com/JerkyTreats/meld/commit/46cd5f174da608fbc8ce1b7da47b6bc84cb49298)
* **prompt_context:** compact filesystem CAS artifacts with workspace cleanup [e9687ab](https://github.com/JerkyTreats/meld/commit/e9687abdca0c8e596825dd9d216ca29726657ac0)
* **context:** stabilize workflow-backed generation reruns [1eb3a83](https://github.com/JerkyTreats/meld/commit/1eb3a83f4a63adafdcc0524632189ec4ce74cb93)
* **context:** prefer final workflow result in context get [5cd8819](https://github.com/JerkyTreats/meld/commit/5cd88193694a93a3d83f7004a80e93b485eef0ee)

### Refactors

* **context:** extract target execution program contract [9663a7c](https://github.com/JerkyTreats/meld/commit/9663a7c8550b7b5caff4776cb1db4157be3098df)
* **workflow:** add target execution facade [8037c9a](https://github.com/JerkyTreats/meld/commit/8037c9a5df48225543c7944b7e6049912e4afd3a)
* **context:** route workflow agents through target execution queue [5ee1b95](https://github.com/JerkyTreats/meld/commit/5ee1b95b535cbe179e28705aebcf0aac66902898)

### Chores

* **scripts:** add release dry run helper [03e556c](https://github.com/JerkyTreats/meld/commit/03e556c78c52148f65f40e75dca68922368411f4)

### Design

* **workflow_bootstrap:** define turn manager spec and phased plan [2ca55a3](https://github.com/JerkyTreats/meld/commit/2ca55a3d9efcf0010494d7927c9a0c2f34cd8e98)
* **turn_manager:** record phase seven verification lock completion [1425bc5](https://github.com/JerkyTreats/meld/commit/1425bc5a5ae8bacf8490652108b3068415328689)
* **workflow:** define cli feedback and capture capability backlog [2d0e110](https://github.com/JerkyTreats/meld/commit/2d0e11015d28bd1d0a0eea80355f460d4fdb29e1)

### Policy

* **storage:** define storage governance with workspace purity first [83934dc](https://github.com/JerkyTreats/meld/commit/83934dcb4a8200d61a3ffe9107b5985a4aadcb11)


## [1.2.0](https://github.com/JerkyTreats/meld/compare/v1.1.5...v1.2.0) — 2026-03-06

### Features

* **workflow:** publish canonical record contracts for turn gate and prompt link [1e47931](https://github.com/JerkyTreats/meld/commit/1e47931da37966757988c4de4e653a6dbbf6c6f3)


## [1.1.5](https://github.com/JerkyTreats/meld/compare/v1.1.4...v1.1.5) — 2026-03-05

### Refactors

* **prompt_context:** persist generation lineage in filesystem CAS [9daf920](https://github.com/JerkyTreats/meld/commit/9daf9203f0dbc07fc32fdd0d53579c4ea83f32a8)
* **prompt_context:** centralize lineage to metadata contract translation [d95bb04](https://github.com/JerkyTreats/meld/commit/d95bb04518df08945e068589a239126bf1447dc9)

### Documentation

* **metadata:** update design README [aeffab1](https://github.com/JerkyTreats/meld/commit/aeffab1fb827a1c0b10f219e0acff99b59325630)


## [1.1.4](https://github.com/JerkyTreats/meld/compare/v1.1.3...v1.1.4) — 2026-03-05

### Refactors

* **metadata:** enforce descriptor driven frame write contracts [24dc847](https://github.com/JerkyTreats/meld/commit/24dc84723be03ebc43d90f05120d05296714c55d)


## [1.1.3](https://github.com/JerkyTreats/meld/compare/v1.1.2...v1.1.3) — 2026-03-04

### Refactors

* **metadata:** split frame key descriptors by owning domain [df86145](https://github.com/JerkyTreats/meld/commit/df86145c6d11cd8107a6874d2db7be8a30c721fa)

### Design

* **workflow-bootstrap:** expand metadata contract specs and planning [afe7637](https://github.com/JerkyTreats/meld/commit/afe76375eee6797d9b48f2670a5cf9756dbf61f4)


## [1.1.2](https://github.com/JerkyTreats/meld/compare/v1.1.1...v1.1.2) — 2026-03-04

### Refactors

* **metadata:** enforce registry-driven frame metadata policy [37df85f](https://github.com/JerkyTreats/meld/commit/37df85f194db9e183fb55015135b541f2a6d980c)


## [1.1.1](https://github.com/JerkyTreats/meld/compare/v1.1.0...v1.1.1) — 2026-03-04

### Bug Fixes

* **ci:** avoid bash regex parser failure in release version scan [2bb543a](https://github.com/JerkyTreats/meld/commit/2bb543aef3e9f7789c7686cceb06a64888d20f49)
* **ci:** remove fragile crates API precheck and make publish idempotent [93ddb97](https://github.com/JerkyTreats/meld/commit/93ddb976ec6c86b08773bcdce012d9397fb88e1b)
* **ci:** remove post publish crates visibility gate [b305a42](https://github.com/JerkyTreats/meld/commit/b305a426bb11089ef134f762c577cf55879664bb)

### Refactors

* **context:** enforce typed frame metadata integrity with prompt compatibility [7deaa49](https://github.com/JerkyTreats/meld/commit/7deaa49031bfaa8cfb768718de496387ada5c8a5)
* **generation:** split queue content processing into orchestration units [2c2088d](https://github.com/JerkyTreats/meld/commit/2c2088dfe307f5d749df085ce2710fd8d027d7e7)

### Documentation

* remove shortlist [e6f612b](https://github.com/JerkyTreats/meld/commit/e6f612b11c010a0ca407696e0f5162d916361381)

### Tests

* **integration:** serialize xdg env mutations for xdg config tests [61012a6](https://github.com/JerkyTreats/meld/commit/61012a6ae4dd8de4f526d2d463b320857413b618)

### CI

* **release:** run direct publish flow and fix lockfileless ci [f647536](https://github.com/JerkyTreats/meld/commit/f647536ca957857cb2ead5dc23e9bb8a9dffabcf)
* **fix:** attempt more reliable workflow [39a202a](https://github.com/JerkyTreats/meld/commit/39a202ac14beb0202157809e0d55cf857a855fd3)

### Policy

* **agents:** require commit policy check before git commit [6a03934](https://github.com/JerkyTreats/meld/commit/6a039346291b2ff9b5439afa677329afb4a5d7ef)



## [1.1.0](https://github.com/JerkyTreats/meld/compare/v1.0.2...v1.1.0) (2026-03-02)


### Features

* **logging:** enable default file logging with cross platform path resolution ([f3cc1ee](https://github.com/JerkyTreats/meld/commit/f3cc1ee012692d6ca23d5740e714e29af9b641e2))


### Bug Fixes

* **context:** ground file generation prompts on source content ([06bd0a2](https://github.com/JerkyTreats/meld/commit/06bd0a21c93ed57b0b8bb6185c1551d468610e16))

## [1.0.2](https://github.com/JerkyTreats/meld/compare/v1.0.1...v1.0.2) (2026-02-27)


### Bug Fixes

* **provider:** add default request wait timeouts ([458a98f](https://github.com/JerkyTreats/meld/commit/458a98f8ec3fc2ef87aeb810ae5e791fab528718))
* **provider:** infer https for local endpoints and prompt local api key ([f3f40c9](https://github.com/JerkyTreats/meld/commit/f3f40c97b4cf87ecfe6de7acaea2e9fabaa9be0a))

## [1.0.1](https://github.com/JerkyTreats/meld/compare/v1.0.0...v1.0.1) (2026-02-26)


### Bug Fixes

* **ci:** attempt to align crates/release-please version ([2b6eaad](https://github.com/JerkyTreats/meld/commit/2b6eaadda9c6ce97efe551e87b5e2dcf4a24f7ac))
* **prompt:** add better prompt of docs-writer ([ba50c85](https://github.com/JerkyTreats/meld/commit/ba50c85b4a5204293018e9039edf2f7b197fbc8d))

## [0.1.1](https://github.com/JerkyTreats/meld/compare/v0.1.0...v0.1.1) (2026-02-25)


### Bug Fixes

* **ci:** correct(?) release repo ([24d05ad](https://github.com/JerkyTreats/meld/commit/24d05ad9ccd18430fcfd827f01e05f54f394ca84))

## 0.1.0 (2026-02-25)


### ⚠ BREAKING CHANGES

* Version 1.0

### Features

* Add baseline tests for refactor ([2df72f8](https://github.com/JerkyTreats/meld/commit/2df72f8377b03bfb5261d55abf6092f64b8c0a5c))
* **agent:** add prompt show and prompt edit commands ([e5dd894](https://github.com/JerkyTreats/meld/commit/e5dd894b9b19fa219b766c666d74560aa4c605a0))
* **context:** Add Context Orchestration ([684b62a](https://github.com/JerkyTreats/meld/commit/684b62a4020c2818925aa83a47293c7020858a33))
* **context:** Add regenerate alias for generate --force --no-recursive ([41f684f](https://github.com/JerkyTreats/meld/commit/41f684fff65a77d4a8f0c38f196d23a7dbf81a25))
* **context:** include child context in directory generation ([26faa5f](https://github.com/JerkyTreats/meld/commit/26faa5fac9c7f09a0871516e4461025201d65e19))
* Remove regeneration feature ([f9b098f](https://github.com/JerkyTreats/meld/commit/f9b098f1a34e402f0a6c2dd884fc1e0eb3b6ab5f))
* Remove synthesis feature ([fe031a9](https://github.com/JerkyTreats/meld/commit/fe031a91403d370268b30b888497aab4d27818c8))
* Version 1.0 ([6e13fe3](https://github.com/JerkyTreats/meld/commit/6e13fe3843c52c8033e2785feab36fac1e717c83))


### Bug Fixes

* Add missing summary event families ([f1d2c72](https://github.com/JerkyTreats/meld/commit/f1d2c7220e3408e133a41c8afa291194d959784b))
* **agent:** show resolved prompt path in agent show output ([2a02681](https://github.com/JerkyTreats/meld/commit/2a02681116785e798a9f681d3fb7f98dba54267b))
* **ci:** deploy to crates.io ([790668e](https://github.com/JerkyTreats/meld/commit/790668ef601d6305adce85c99a4e7bc64fa9603a))
* **cli:** make context generate blocking-only ([e65970a](https://github.com/JerkyTreats/meld/commit/e65970a9af5bdae4d465e267dda88915af5b5c7d))
* **cli:** resolve relative paths ([887320f](https://github.com/JerkyTreats/meld/commit/887320f5737850698048ef0712489f477ee3b66b))
* **diagnostics:** surface generation failures and local auth warnings ([02f5824](https://github.com/JerkyTreats/meld/commit/02f58240afa694fc5d82682857c853c90cb328ae))
* **observability:** add typed summaries and provider test lifecycle events ([d3e12e3](https://github.com/JerkyTreats/meld/commit/d3e12e3c7297cc644864e53c4a41a6872ae55712))
* **observability:** align timestamps and bound command summaries ([b7d8a40](https://github.com/JerkyTreats/meld/commit/b7d8a4046d9c652a962301a43c2bbcc6b3496b6d))
* **observability:** include context-generate path identity fields ([2e9e8f3](https://github.com/JerkyTreats/meld/commit/2e9e8f3c40dbea7157a6d95aeeb45f1029ab7d56))
* **queue:** coalesce queued and in-flight generation duplicates ([cefe949](https://github.com/JerkyTreats/meld/commit/cefe949c7c61c4bfc7258b6373e981e0d4eac928))
* **queue:** emit per-item enqueue events for batch requests ([f9900db](https://github.com/JerkyTreats/meld/commit/f9900dbdc931576e1d3d5da8535b84a7848b9021))
* **scan:** emit batched scan_progress events by node count ([251f9a8](https://github.com/JerkyTreats/meld/commit/251f9a8347f817b270d81631e4f5a0ea6e5c72de))
* **xdg:** preserve agent entries and surface config validation errors ([1ce8cd9](https://github.com/JerkyTreats/meld/commit/1ce8cd9d85268d86563036643155505028a77b15))
