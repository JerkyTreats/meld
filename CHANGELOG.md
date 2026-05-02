# Changelog

## [2.5.0](https://github.com/JerkyTreats/meld/compare/v2.4.0...v2.5.0) — 2026-05-02

### Features

* **world_state:** add graph contracts store and query surface [5ae3305](https://github.com/JerkyTreats/meld/commit/5ae3305533f85ab542c877978839208e028505d0)
* **world_state:** reduce execution facts into current claim state [25331c4](https://github.com/JerkyTreats/meld/commit/25331c4a820e3213ca36ec98b649d098c83bd488)
* **context:** publish canonical frame and head traversal events [323d520](https://github.com/JerkyTreats/meld/commit/323d520aea04f348f777a0bcbbf2eb864661a759)
* **workspace_fs:** publish canonical workspace traversal facts [148d642](https://github.com/JerkyTreats/meld/commit/148d6428ee8dd57a2cf59511505e4eddfcee501b)
* **world_state:** add traversal store indexes and graph walk query [522439d](https://github.com/JerkyTreats/meld/commit/522439d9397f48321012c3c2860de6e1a0de3f7f)
* **world_state:** route legacy claim reads through traversal indexes [1ac669f](https://github.com/JerkyTreats/meld/commit/1ac669f0d2743cbe232f7d7f04c959e0fa802821)
* **world_state:** bootstrap graph reducer runtime [e56fc1b](https://github.com/JerkyTreats/meld/commit/e56fc1b0ec5205518fac921ed6db0c8f0e2d8f5e)
* **roots:** register active workspaces with migration metadata [27615e6](https://github.com/JerkyTreats/meld/commit/27615e6629a024944545e938383a9b503e0f137b)
* **branches:** add dormant branch discovery and migration flows [8b8d1e5](https://github.com/JerkyTreats/meld/commit/8b8d1e53a0e53deae840c49416069af1e245f9ad)
* **branches:** add federated branch graph queries [4767c99](https://github.com/JerkyTreats/meld/commit/4767c99ba5975362dd65dbde2cf17d3a97fc2514)
* **workspace:** publish canonical workspace facts from promoted watch batches [352b956](https://github.com/JerkyTreats/meld/commit/352b95627491a007c2c2fc88fe609861c2d35c13)
* **world_state:** persist derived traversal facts through idempotent spine append [21b6181](https://github.com/JerkyTreats/meld/commit/21b61811d098e63f5c95fa83b144d0406bfe34d2)
* **workflow:** resolve task path outputs through traversal artifact anchors [f9f0cfe](https://github.com/JerkyTreats/meld/commit/f9f0cfe6600bf296d05784cc3feed722b74c1e7a)

### Bug Fixes

* **refactor:** rename roots -> branches [3d38a95](https://github.com/JerkyTreats/meld/commit/3d38a95f8050a2cde65d8aa2cc092da6cfccd605)
* **workflow:** reuse completed task path threads through traversal state [c694090](https://github.com/JerkyTreats/meld/commit/c69409058edc5a3412a1e5290c67704e7d776040)
* **context:** honor force for frame regeneration [f79329b](https://github.com/JerkyTreats/meld/commit/f79329b7ffd9f6a8f74fe2ad7558869bfe9f48b2)
* **context:** log queue worker shutdown failures [bdc40ea](https://github.com/JerkyTreats/meld/commit/bdc40eae6f578ff62a6c760bae3ed0a39fa8b103)
* **meld-execution:** vendor built-in task package yaml [6c7afa6](https://github.com/JerkyTreats/meld/commit/6c7afa6ef801e3c34012df24a8cd95301e607838)
* **ci:** cargo lock flow [e50bf6f](https://github.com/JerkyTreats/meld/commit/e50bf6f8e77e1d6ad120c4e5b493782c398a80ff)
* **ci:** skip existing package [43c00c7](https://github.com/JerkyTreats/meld/commit/43c00c790e148bf9fe58fa013d0f0bfec8ba78f4)
* **ci:** skip existing package again [8920061](https://github.com/JerkyTreats/meld/commit/892006111290c45d0aa0aa1f6c85938eb91bd296)
* **ci:** CI ordering [618b654](https://github.com/JerkyTreats/meld/commit/618b6547f9a9df8e8c14224afe810d11abfd2ddd)
* **ci:** gitignore [47cc074](https://github.com/JerkyTreats/meld/commit/47cc074d1adee12e0dd1ea398e3e3b02a9417e8e)

### Refactors

* **telemetry:** add runtime-wide spine envelope and store [bf5c927](https://github.com/JerkyTreats/meld/commit/bf5c9279c22f3117fb15a51886e503835e6e027e)
* **execution:** define domain-owned canonical event contracts [6af5b10](https://github.com/JerkyTreats/meld/commit/6af5b10946cacddb766f41701b14214f49275016)
* **execution:** publish canonical events from owning domains [50fa167](https://github.com/JerkyTreats/meld/commit/50fa16704fd755364e5bb0bd743f03acd37e3650)
* **control:** reduce spine events into execution projection [0018f22](https://github.com/JerkyTreats/meld/commit/0018f22ed53b1428b9ab6c32f84af9d88f4ae72c)
* **telemetry:** make telemetry downstream of the spine [8a64160](https://github.com/JerkyTreats/meld/commit/8a64160ff6e06d0ac7e498dcd64b11eccdc960ea)
* **execution:** finalize spine execution slice [f37d891](https://github.com/JerkyTreats/meld/commit/f37d891bf74f6296b21def035c8d75d4bb0e5c33)
* **telemetry:** add object refs and relations to spine events [a7ee266](https://github.com/JerkyTreats/meld/commit/a7ee26666c91730f76fc47f3e7c00d599df8072b)
* **execution:** attach object refs to canonical execution events [07a175f](https://github.com/JerkyTreats/meld/commit/07a175f1b8eae99c1cf4b2ba5230c237b84660a8)
* **world_state:** add traversal-native contracts and compatibility skeleton [0143f6a](https://github.com/JerkyTreats/meld/commit/0143f6a077b59dba69955308616504b8d77714f3)
* **execution:** complete traversal refs for task and workflow events [9f1e9d8](https://github.com/JerkyTreats/meld/commit/9f1e9d827748dd1df9753f9ace27645f8a9a077e)
* **world_state:** rename traversal module to graph [099100d](https://github.com/JerkyTreats/meld/commit/099100d9a8ba1fb3bd4beb19938c056013d1dc8a)
* **roots:** add branch-aware metadata compatibility [947e842](https://github.com/JerkyTreats/meld/commit/947e842156a755b55ea3af2d5fd9968c17c1ee6a)
* **roots:** add internal branch handle runtime seam [e2bd508](https://github.com/JerkyTreats/meld/commit/e2bd5089428e652bd6a9101bc8ba7fd9bc3c5c19)
* **roots:** isolate legacy root naming behind branch aliases [3cf6159](https://github.com/JerkyTreats/meld/commit/3cf61599ca33d2ad86bca906ae3cb648e60226da)
* **branches:** add canonical branches module and cli alias [28f91f6](https://github.com/JerkyTreats/meld/commit/28f91f6dea63e1c304fb9101def70a3b5db03768)
* **events:** add canonical event contracts and compatibility shims [0d6a85e](https://github.com/JerkyTreats/meld/commit/0d6a85e3960b0ab66a9e10ab024c2f925d3e3e24)
* **session:** split minimal session lifecycle from telemetry [6e35861](https://github.com/JerkyTreats/meld/commit/6e35861bfdaaefc9caa6ca6ae7f8d370af437908)
* **events:** extract canonical append and replay runtime [8f62b12](https://github.com/JerkyTreats/meld/commit/8f62b12931dbbbd16cb662ad3e7eb77433270d2b)
* **events:** cut producers over to canonical event runtime [963b9c9](https://github.com/JerkyTreats/meld/commit/963b9c9c74f1dbef685175354a171b8ce2ff2258)
* **events:** retarget replay consumers to canonical store [d390500](https://github.com/JerkyTreats/meld/commit/d390500bcd8534e0267e050e8044fa8e69fac504)
* **telemetry:** reduce telemetry to downstream compatibility [9146b12](https://github.com/JerkyTreats/meld/commit/9146b120d4b79002b041df9603e15f143d499de0)
* **events:** finalize canonical ownership extraction [2191c03](https://github.com/JerkyTreats/meld/commit/2191c035f8bc25745d079c7138c2d0248ee8d537)
* **events:** preserve canonical spine history across session cleanup [a239e47](https://github.com/JerkyTreats/meld/commit/a239e47e57d718cd1f76325d19a3c2ce57125679)
* **branches:** annotate federated traversal results with branch provenance [f77357f](https://github.com/JerkyTreats/meld/commit/f77357f6fb04bb4fb9630ba87e2d3989d9cddb93)
* **world_state:** move graph source reduction into domain reducers [45606b9](https://github.com/JerkyTreats/meld/commit/45606b9ede44457d4b847354c51b5de848cb0f35)
* **events:** separate session lifecycle from event store [037eab6](https://github.com/JerkyTreats/meld/commit/037eab62a957c86c1e8b9dd039123b2ec01fc989)
* **world-state:** route reads through world model queries [7eb71d4](https://github.com/JerkyTreats/meld/commit/7eb71d46e60e1e464b6e1413162b4cbea72a9c07)
* **execution:** extract runtime ports and request contracts [f78708c](https://github.com/JerkyTreats/meld/commit/f78708c8ac00f156f82d1ac5c076493fcc0ec79c)
* **workflow:** cut runtime through root assembly [9bf4c5a](https://github.com/JerkyTreats/meld/commit/9bf4c5a32df03cd3131ff802485e76ea5d8e9280)
* **core:** seal legacy runtime surfaces behind compat [1983b18](https://github.com/JerkyTreats/meld/commit/1983b18ca079762de8dab4282d9afbb4211c459b)
* **core:** remove dead in-tree authority copies [1f15f25](https://github.com/JerkyTreats/meld/commit/1f15f2510874ac95f96593955ba4ef8658c15213)
* **execution:** extract execution authority contracts [c994026](https://github.com/JerkyTreats/meld/commit/c994026f5aac553f41f15e4645cfc27a52a3d2a7)
* **execution:** move workflow and task contracts into crate [411c8eb](https://github.com/JerkyTreats/meld/commit/411c8ebf26113a0ca0b3776df580a25efc252367)
* **execution:** move provider and generation dto contracts into crate [6756474](https://github.com/JerkyTreats/meld/commit/67564746a95db9c5bddc7ddb4e7c5746885c390e)
* **execution:** replace queue event context with runtime ports [f73ac43](https://github.com/JerkyTreats/meld/commit/f73ac43a3d1e9affbc645741e7b531f4a4cac6df)
* **execution:** route lineage and metadata through runtime ports [8607d39](https://github.com/JerkyTreats/meld/commit/8607d397d51e83555bfd8b7471ccdd6560886720)
* **execution:** move task and workflow event ownership into crate [0809bf7](https://github.com/JerkyTreats/meld/commit/0809bf77d8d40cf20097140ce81f5ce586bbeb43)
* **capability:** move shared catalog payloads and registry contracts into crate [eb85a56](https://github.com/JerkyTreats/meld/commit/eb85a56a95a47e1b446f5a753f6099c5516bd45b)
* **task:** move core task engine into execution crate [28e14b2](https://github.com/JerkyTreats/meld/commit/28e14b26e42beba072ed63d1a740046741960ad4)
* **task:** retarget package trigger bindings to execution contracts [44c8bcc](https://github.com/JerkyTreats/meld/commit/44c8bcc479136e7dde8de342a461f4a659861b8d)
* **execution:** add execution read models and world model ports [5193006](https://github.com/JerkyTreats/meld/commit/5193006ea7d0fd49be47a96e0dfb2b9fa878951a)
* **execution:** move traversal and publish contracts into crate [f856bff](https://github.com/JerkyTreats/meld/commit/f856bffa82059e899aabf78a16d28fb65bd9925b)
* **task:** move package contracts and lowering into execution crate [63179ef](https://github.com/JerkyTreats/meld/commit/63179efb3aa0b6bdb1158659de65d2eb96c5a5e4)
* **task:** move package discovery into execution crate [2893812](https://github.com/JerkyTreats/meld/commit/28938123341da95e919e0e96b92fb90001ac8199)
* **task:** move package preparation into execution crate [4a13498](https://github.com/JerkyTreats/meld/commit/4a13498916d96d8a197f6cffd8baa7cc1f63d65c)
* **task:** move template materialization into execution crate [879424c](https://github.com/JerkyTreats/meld/commit/879424c5be1d6517c63a04579dde6a779de6e4b4)
* **task:** add extracted expansion compiler registry [5dc7da1](https://github.com/JerkyTreats/meld/commit/5dc7da13ab666dfb6aa72d22ab72cee09f1fa789)
* **task:** route live expansion dispatch through compiler registry [d9961d7](https://github.com/JerkyTreats/meld/commit/d9961d712f6eac29af2cfcdd1c2e12d8d39c7f0a)
* **capability:** converge registry ownership on meld-execution [c469ee5](https://github.com/JerkyTreats/meld/commit/c469ee5666499f7465ce74dbccaee50268b090b3)
* **task:** remove dead root package ballast [1857105](https://github.com/JerkyTreats/meld/commit/1857105e380acf3d413956a441585a56e6831410)
* **task:** reduce root package wrappers to adapter seams [433659f](https://github.com/JerkyTreats/meld/commit/433659f47eb75f2692d418c48ca20192748bb974)
* **task:** seal root runtime and expansion adapters [143f3ad](https://github.com/JerkyTreats/meld/commit/143f3adbf8ba049f3a96cb9cf42da0b0e17e0252)
* **task:** remove template compatibility shim [b8d9450](https://github.com/JerkyTreats/meld/commit/b8d94506bc93459022df3f8155cb63a65fdef938)
* **execution:** add workflow lineage metadata and progress contracts [8ed2c3c](https://github.com/JerkyTreats/meld/commit/8ed2c3c5956aada5bbe666fd0a0e27cf89e37c62)
* **workflow:** move resolver gates and normalization into execution crate [cd826ac](https://github.com/JerkyTreats/meld/commit/cd826ac7e8e93d5d811a8c2ce5df1f5420a003f9)
* **workflow:** move records and state store into execution crate [9ce1b9d](https://github.com/JerkyTreats/meld/commit/9ce1b9dae72e7fd30e91a62965457577883d7b1c)
* **workflow:** inject task path capability bundle [d225cdb](https://github.com/JerkyTreats/meld/commit/d225cdb8875b18442b3435f80730a1a805fef97c)
* **workflow:** move executor ownership into execution crate [28419b6](https://github.com/JerkyTreats/meld/commit/28419b6a937554ef90fbc342c9e74443fa5433f3)
* **workflow:** decompose extracted executor [529c409](https://github.com/JerkyTreats/meld/commit/529c409b8e091634b112a3c31142b280c3e5c28c)
* **workflow:** group executor arguments into contexts [a61e619](https://github.com/JerkyTreats/meld/commit/a61e6190cf344fa841aa6d803cc550c17279f6f9)

### Documentation

* add spine graph completion and workflow references [987b467](https://github.com/JerkyTreats/meld/commit/987b4671fb0dc26a798dfebe255839686a851d63)
* **design:** cleanup cognitive_architecture [a1735e7](https://github.com/JerkyTreats/meld/commit/a1735e76f5cef22ea80caa6970828ae6ba1db117)
* **design:** cognitive_archictecture research/refinement [481c05a](https://github.com/JerkyTreats/meld/commit/481c05a7072aff361bb7dc90311427fee7933f62)

### Tests

* **world_state:** lock replay and query acceptance coverage [8613e2f](https://github.com/JerkyTreats/meld/commit/8613e2f584e9116d43859067779f8842c141bf15)
* **world_state:** lock full traversal acceptance coverage [44427a5](https://github.com/JerkyTreats/meld/commit/44427a543b784f50e0f373d3162c884bebc2251c)
* **crates:** add extracted crate contract tests [77fc600](https://github.com/JerkyTreats/meld/commit/77fc600249b7e871fa95c8e0879d2a68506f5450)

### Build

* **workspace:** extract events and world model crates [505901e](https://github.com/JerkyTreats/meld/commit/505901e673e38a672ca62754a2920cb346cecc67)

### CI

* **release:** publish workspace crates in dependency order [220ed0e](https://github.com/JerkyTreats/meld/commit/220ed0e0e4bc42fd67c14d3d357bc3da8ab2c2c4)

### Chores

* apply repository formatter output [11a5125](https://github.com/JerkyTreats/meld/commit/11a5125880e5c647a638160b8e5a916541dd593d)

### Design

* **cognitive_architecture:** define event spine execution slice [d736f91](https://github.com/JerkyTreats/meld/commit/d736f91919a1234e509c655be4c0280c45668d69)
* **cognitive_architecture:** define temporal fact graph implementation slice [804c6c6](https://github.com/JerkyTreats/meld/commit/804c6c60532718181cce8beb9b1397a01f2acfb3)
* **cognitive_architecture:** split world_state into traversal and belief [223e363](https://github.com/JerkyTreats/meld/commit/223e3638c72df10644d1e42f69eb760b863e93b6)
* **events:** define event extraction execution plan [84ac3cf](https://github.com/JerkyTreats/meld/commit/84ac3cfdb450ac7f21540a44225aa1219e6f287b)
* **world_state:** record spine graph completion evidence and close checkpoints [1954260](https://github.com/JerkyTreats/meld/commit/19542608ccc6746033ebae077d42fde182699405)
* **cognitive-architecture:** publish split baseline artifacts [25b13cc](https://github.com/JerkyTreats/meld/commit/25b13cc4c9266364fd19282050be32626df00268)
* **cognitive-architecture:** expand split architecture docs [b74c2b1](https://github.com/JerkyTreats/meld/commit/b74c2b1b738a14bdbc9cae4db0fa9487494f6c9d)

### Policy

* **governance:** define compatibility shim lifecycle [75856a0](https://github.com/JerkyTreats/meld/commit/75856a0d64847bd4e59dd45347e346e0827338fc)
* **workflow:** require formatter evidence for complex change gates [9f09029](https://github.com/JerkyTreats/meld/commit/9f09029a3587cf5b09f7603934241418995903f4)
* **agents:** index assessment by domain governance [a9a48a6](https://github.com/JerkyTreats/meld/commit/a9a48a658f2b721965453fa7930dda712c0e9646)


## [2.5.0](https://github.com/JerkyTreats/meld/compare/v2.4.0...v2.5.0) — 2026-05-02

### Features

* **world_state:** add graph contracts store and query surface [5ae3305](https://github.com/JerkyTreats/meld/commit/5ae3305533f85ab542c877978839208e028505d0)
* **world_state:** reduce execution facts into current claim state [25331c4](https://github.com/JerkyTreats/meld/commit/25331c4a820e3213ca36ec98b649d098c83bd488)
* **context:** publish canonical frame and head traversal events [323d520](https://github.com/JerkyTreats/meld/commit/323d520aea04f348f777a0bcbbf2eb864661a759)
* **workspace_fs:** publish canonical workspace traversal facts [148d642](https://github.com/JerkyTreats/meld/commit/148d6428ee8dd57a2cf59511505e4eddfcee501b)
* **world_state:** add traversal store indexes and graph walk query [522439d](https://github.com/JerkyTreats/meld/commit/522439d9397f48321012c3c2860de6e1a0de3f7f)
* **world_state:** route legacy claim reads through traversal indexes [1ac669f](https://github.com/JerkyTreats/meld/commit/1ac669f0d2743cbe232f7d7f04c959e0fa802821)
* **world_state:** bootstrap graph reducer runtime [e56fc1b](https://github.com/JerkyTreats/meld/commit/e56fc1b0ec5205518fac921ed6db0c8f0e2d8f5e)
* **roots:** register active workspaces with migration metadata [27615e6](https://github.com/JerkyTreats/meld/commit/27615e6629a024944545e938383a9b503e0f137b)
* **branches:** add dormant branch discovery and migration flows [8b8d1e5](https://github.com/JerkyTreats/meld/commit/8b8d1e53a0e53deae840c49416069af1e245f9ad)
* **branches:** add federated branch graph queries [4767c99](https://github.com/JerkyTreats/meld/commit/4767c99ba5975362dd65dbde2cf17d3a97fc2514)
* **workspace:** publish canonical workspace facts from promoted watch batches [352b956](https://github.com/JerkyTreats/meld/commit/352b95627491a007c2c2fc88fe609861c2d35c13)
* **world_state:** persist derived traversal facts through idempotent spine append [21b6181](https://github.com/JerkyTreats/meld/commit/21b61811d098e63f5c95fa83b144d0406bfe34d2)
* **workflow:** resolve task path outputs through traversal artifact anchors [f9f0cfe](https://github.com/JerkyTreats/meld/commit/f9f0cfe6600bf296d05784cc3feed722b74c1e7a)

### Bug Fixes

* **refactor:** rename roots -> branches [3d38a95](https://github.com/JerkyTreats/meld/commit/3d38a95f8050a2cde65d8aa2cc092da6cfccd605)
* **workflow:** reuse completed task path threads through traversal state [c694090](https://github.com/JerkyTreats/meld/commit/c69409058edc5a3412a1e5290c67704e7d776040)
* **context:** honor force for frame regeneration [f79329b](https://github.com/JerkyTreats/meld/commit/f79329b7ffd9f6a8f74fe2ad7558869bfe9f48b2)
* **context:** log queue worker shutdown failures [bdc40ea](https://github.com/JerkyTreats/meld/commit/bdc40eae6f578ff62a6c760bae3ed0a39fa8b103)

### Refactors

* **telemetry:** add runtime-wide spine envelope and store [bf5c927](https://github.com/JerkyTreats/meld/commit/bf5c9279c22f3117fb15a51886e503835e6e027e)
* **execution:** define domain-owned canonical event contracts [6af5b10](https://github.com/JerkyTreats/meld/commit/6af5b10946cacddb766f41701b14214f49275016)
* **execution:** publish canonical events from owning domains [50fa167](https://github.com/JerkyTreats/meld/commit/50fa16704fd755364e5bb0bd743f03acd37e3650)
* **control:** reduce spine events into execution projection [0018f22](https://github.com/JerkyTreats/meld/commit/0018f22ed53b1428b9ab6c32f84af9d88f4ae72c)
* **telemetry:** make telemetry downstream of the spine [8a64160](https://github.com/JerkyTreats/meld/commit/8a64160ff6e06d0ac7e498dcd64b11eccdc960ea)
* **execution:** finalize spine execution slice [f37d891](https://github.com/JerkyTreats/meld/commit/f37d891bf74f6296b21def035c8d75d4bb0e5c33)
* **telemetry:** add object refs and relations to spine events [a7ee266](https://github.com/JerkyTreats/meld/commit/a7ee26666c91730f76fc47f3e7c00d599df8072b)
* **execution:** attach object refs to canonical execution events [07a175f](https://github.com/JerkyTreats/meld/commit/07a175f1b8eae99c1cf4b2ba5230c237b84660a8)
* **world_state:** add traversal-native contracts and compatibility skeleton [0143f6a](https://github.com/JerkyTreats/meld/commit/0143f6a077b59dba69955308616504b8d77714f3)
* **execution:** complete traversal refs for task and workflow events [9f1e9d8](https://github.com/JerkyTreats/meld/commit/9f1e9d827748dd1df9753f9ace27645f8a9a077e)
* **world_state:** rename traversal module to graph [099100d](https://github.com/JerkyTreats/meld/commit/099100d9a8ba1fb3bd4beb19938c056013d1dc8a)
* **roots:** add branch-aware metadata compatibility [947e842](https://github.com/JerkyTreats/meld/commit/947e842156a755b55ea3af2d5fd9968c17c1ee6a)
* **roots:** add internal branch handle runtime seam [e2bd508](https://github.com/JerkyTreats/meld/commit/e2bd5089428e652bd6a9101bc8ba7fd9bc3c5c19)
* **roots:** isolate legacy root naming behind branch aliases [3cf6159](https://github.com/JerkyTreats/meld/commit/3cf61599ca33d2ad86bca906ae3cb648e60226da)
* **branches:** add canonical branches module and cli alias [28f91f6](https://github.com/JerkyTreats/meld/commit/28f91f6dea63e1c304fb9101def70a3b5db03768)
* **events:** add canonical event contracts and compatibility shims [0d6a85e](https://github.com/JerkyTreats/meld/commit/0d6a85e3960b0ab66a9e10ab024c2f925d3e3e24)
* **session:** split minimal session lifecycle from telemetry [6e35861](https://github.com/JerkyTreats/meld/commit/6e35861bfdaaefc9caa6ca6ae7f8d370af437908)
* **events:** extract canonical append and replay runtime [8f62b12](https://github.com/JerkyTreats/meld/commit/8f62b12931dbbbd16cb662ad3e7eb77433270d2b)
* **events:** cut producers over to canonical event runtime [963b9c9](https://github.com/JerkyTreats/meld/commit/963b9c9c74f1dbef685175354a171b8ce2ff2258)
* **events:** retarget replay consumers to canonical store [d390500](https://github.com/JerkyTreats/meld/commit/d390500bcd8534e0267e050e8044fa8e69fac504)
* **telemetry:** reduce telemetry to downstream compatibility [9146b12](https://github.com/JerkyTreats/meld/commit/9146b120d4b79002b041df9603e15f143d499de0)
* **events:** finalize canonical ownership extraction [2191c03](https://github.com/JerkyTreats/meld/commit/2191c035f8bc25745d079c7138c2d0248ee8d537)
* **events:** preserve canonical spine history across session cleanup [a239e47](https://github.com/JerkyTreats/meld/commit/a239e47e57d718cd1f76325d19a3c2ce57125679)
* **branches:** annotate federated traversal results with branch provenance [f77357f](https://github.com/JerkyTreats/meld/commit/f77357f6fb04bb4fb9630ba87e2d3989d9cddb93)
* **world_state:** move graph source reduction into domain reducers [45606b9](https://github.com/JerkyTreats/meld/commit/45606b9ede44457d4b847354c51b5de848cb0f35)
* **events:** separate session lifecycle from event store [037eab6](https://github.com/JerkyTreats/meld/commit/037eab62a957c86c1e8b9dd039123b2ec01fc989)
* **world-state:** route reads through world model queries [7eb71d4](https://github.com/JerkyTreats/meld/commit/7eb71d46e60e1e464b6e1413162b4cbea72a9c07)
* **execution:** extract runtime ports and request contracts [f78708c](https://github.com/JerkyTreats/meld/commit/f78708c8ac00f156f82d1ac5c076493fcc0ec79c)
* **workflow:** cut runtime through root assembly [9bf4c5a](https://github.com/JerkyTreats/meld/commit/9bf4c5a32df03cd3131ff802485e76ea5d8e9280)
* **core:** seal legacy runtime surfaces behind compat [1983b18](https://github.com/JerkyTreats/meld/commit/1983b18ca079762de8dab4282d9afbb4211c459b)
* **core:** remove dead in-tree authority copies [1f15f25](https://github.com/JerkyTreats/meld/commit/1f15f2510874ac95f96593955ba4ef8658c15213)
* **execution:** extract execution authority contracts [c994026](https://github.com/JerkyTreats/meld/commit/c994026f5aac553f41f15e4645cfc27a52a3d2a7)
* **execution:** move workflow and task contracts into crate [411c8eb](https://github.com/JerkyTreats/meld/commit/411c8ebf26113a0ca0b3776df580a25efc252367)
* **execution:** move provider and generation dto contracts into crate [6756474](https://github.com/JerkyTreats/meld/commit/67564746a95db9c5bddc7ddb4e7c5746885c390e)
* **execution:** replace queue event context with runtime ports [f73ac43](https://github.com/JerkyTreats/meld/commit/f73ac43a3d1e9affbc645741e7b531f4a4cac6df)
* **execution:** route lineage and metadata through runtime ports [8607d39](https://github.com/JerkyTreats/meld/commit/8607d397d51e83555bfd8b7471ccdd6560886720)
* **execution:** move task and workflow event ownership into crate [0809bf7](https://github.com/JerkyTreats/meld/commit/0809bf77d8d40cf20097140ce81f5ce586bbeb43)
* **capability:** move shared catalog payloads and registry contracts into crate [eb85a56](https://github.com/JerkyTreats/meld/commit/eb85a56a95a47e1b446f5a753f6099c5516bd45b)
* **task:** move core task engine into execution crate [28e14b2](https://github.com/JerkyTreats/meld/commit/28e14b26e42beba072ed63d1a740046741960ad4)
* **task:** retarget package trigger bindings to execution contracts [44c8bcc](https://github.com/JerkyTreats/meld/commit/44c8bcc479136e7dde8de342a461f4a659861b8d)
* **execution:** add execution read models and world model ports [5193006](https://github.com/JerkyTreats/meld/commit/5193006ea7d0fd49be47a96e0dfb2b9fa878951a)
* **execution:** move traversal and publish contracts into crate [f856bff](https://github.com/JerkyTreats/meld/commit/f856bffa82059e899aabf78a16d28fb65bd9925b)
* **task:** move package contracts and lowering into execution crate [63179ef](https://github.com/JerkyTreats/meld/commit/63179efb3aa0b6bdb1158659de65d2eb96c5a5e4)
* **task:** move package discovery into execution crate [2893812](https://github.com/JerkyTreats/meld/commit/28938123341da95e919e0e96b92fb90001ac8199)
* **task:** move package preparation into execution crate [4a13498](https://github.com/JerkyTreats/meld/commit/4a13498916d96d8a197f6cffd8baa7cc1f63d65c)
* **task:** move template materialization into execution crate [879424c](https://github.com/JerkyTreats/meld/commit/879424c5be1d6517c63a04579dde6a779de6e4b4)
* **task:** add extracted expansion compiler registry [5dc7da1](https://github.com/JerkyTreats/meld/commit/5dc7da13ab666dfb6aa72d22ab72cee09f1fa789)
* **task:** route live expansion dispatch through compiler registry [d9961d7](https://github.com/JerkyTreats/meld/commit/d9961d712f6eac29af2cfcdd1c2e12d8d39c7f0a)
* **capability:** converge registry ownership on meld-execution [c469ee5](https://github.com/JerkyTreats/meld/commit/c469ee5666499f7465ce74dbccaee50268b090b3)
* **task:** remove dead root package ballast [1857105](https://github.com/JerkyTreats/meld/commit/1857105e380acf3d413956a441585a56e6831410)
* **task:** reduce root package wrappers to adapter seams [433659f](https://github.com/JerkyTreats/meld/commit/433659f47eb75f2692d418c48ca20192748bb974)
* **task:** seal root runtime and expansion adapters [143f3ad](https://github.com/JerkyTreats/meld/commit/143f3adbf8ba049f3a96cb9cf42da0b0e17e0252)
* **task:** remove template compatibility shim [b8d9450](https://github.com/JerkyTreats/meld/commit/b8d94506bc93459022df3f8155cb63a65fdef938)
* **execution:** add workflow lineage metadata and progress contracts [8ed2c3c](https://github.com/JerkyTreats/meld/commit/8ed2c3c5956aada5bbe666fd0a0e27cf89e37c62)
* **workflow:** move resolver gates and normalization into execution crate [cd826ac](https://github.com/JerkyTreats/meld/commit/cd826ac7e8e93d5d811a8c2ce5df1f5420a003f9)
* **workflow:** move records and state store into execution crate [9ce1b9d](https://github.com/JerkyTreats/meld/commit/9ce1b9dae72e7fd30e91a62965457577883d7b1c)
* **workflow:** inject task path capability bundle [d225cdb](https://github.com/JerkyTreats/meld/commit/d225cdb8875b18442b3435f80730a1a805fef97c)
* **workflow:** move executor ownership into execution crate [28419b6](https://github.com/JerkyTreats/meld/commit/28419b6a937554ef90fbc342c9e74443fa5433f3)
* **workflow:** decompose extracted executor [529c409](https://github.com/JerkyTreats/meld/commit/529c409b8e091634b112a3c31142b280c3e5c28c)
* **workflow:** group executor arguments into contexts [a61e619](https://github.com/JerkyTreats/meld/commit/a61e6190cf344fa841aa6d803cc550c17279f6f9)

### Documentation

* add spine graph completion and workflow references [987b467](https://github.com/JerkyTreats/meld/commit/987b4671fb0dc26a798dfebe255839686a851d63)
* **design:** cleanup cognitive_architecture [a1735e7](https://github.com/JerkyTreats/meld/commit/a1735e76f5cef22ea80caa6970828ae6ba1db117)
* **design:** cognitive_archictecture research/refinement [481c05a](https://github.com/JerkyTreats/meld/commit/481c05a7072aff361bb7dc90311427fee7933f62)

### Tests

* **world_state:** lock replay and query acceptance coverage [8613e2f](https://github.com/JerkyTreats/meld/commit/8613e2f584e9116d43859067779f8842c141bf15)
* **world_state:** lock full traversal acceptance coverage [44427a5](https://github.com/JerkyTreats/meld/commit/44427a543b784f50e0f373d3162c884bebc2251c)
* **crates:** add extracted crate contract tests [77fc600](https://github.com/JerkyTreats/meld/commit/77fc600249b7e871fa95c8e0879d2a68506f5450)

### Build

* **workspace:** extract events and world model crates [505901e](https://github.com/JerkyTreats/meld/commit/505901e673e38a672ca62754a2920cb346cecc67)

### CI

* **release:** publish workspace crates in dependency order [220ed0e](https://github.com/JerkyTreats/meld/commit/220ed0e0e4bc42fd67c14d3d357bc3da8ab2c2c4)

### Chores

* apply repository formatter output [11a5125](https://github.com/JerkyTreats/meld/commit/11a5125880e5c647a638160b8e5a916541dd593d)

### Design

* **cognitive_architecture:** define event spine execution slice [d736f91](https://github.com/JerkyTreats/meld/commit/d736f91919a1234e509c655be4c0280c45668d69)
* **cognitive_architecture:** define temporal fact graph implementation slice [804c6c6](https://github.com/JerkyTreats/meld/commit/804c6c60532718181cce8beb9b1397a01f2acfb3)
* **cognitive_architecture:** split world_state into traversal and belief [223e363](https://github.com/JerkyTreats/meld/commit/223e3638c72df10644d1e42f69eb760b863e93b6)
* **events:** define event extraction execution plan [84ac3cf](https://github.com/JerkyTreats/meld/commit/84ac3cfdb450ac7f21540a44225aa1219e6f287b)
* **world_state:** record spine graph completion evidence and close checkpoints [1954260](https://github.com/JerkyTreats/meld/commit/19542608ccc6746033ebae077d42fde182699405)
* **cognitive-architecture:** publish split baseline artifacts [25b13cc](https://github.com/JerkyTreats/meld/commit/25b13cc4c9266364fd19282050be32626df00268)
* **cognitive-architecture:** expand split architecture docs [b74c2b1](https://github.com/JerkyTreats/meld/commit/b74c2b1b738a14bdbc9cae4db0fa9487494f6c9d)

### Policy

* **governance:** define compatibility shim lifecycle [75856a0](https://github.com/JerkyTreats/meld/commit/75856a0d64847bd4e59dd45347e346e0827338fc)
* **workflow:** require formatter evidence for complex change gates [9f09029](https://github.com/JerkyTreats/meld/commit/9f09029a3587cf5b09f7603934241418995903f4)
* **agents:** index assessment by domain governance [a9a48a6](https://github.com/JerkyTreats/meld/commit/a9a48a658f2b721965453fa7930dda712c0e9646)


## [2.4.0](https://github.com/JerkyTreats/meld/compare/v2.3.1...v2.4.0) — 2026-04-12

### Features

* **capability:** add capability to write files, wire to docs_writer_v2 [8835cc1](https://github.com/JerkyTreats/meld/commit/8835cc1b1daed2464b65a75c3cec4de4e7f3c62a)

### Refactors

* **cli:** move domain command routing into domain tooling [e554e0d](https://github.com/JerkyTreats/meld/commit/e554e0d73a50efd8ebe10aa9869aa93a41b25ebd)

### Documentation

* **completed:** move capabilities to completed [f09f383](https://github.com/JerkyTreats/meld/commit/f09f3830af885382f41b1533fd3e65ac71e7bdfd)
* **design:** add cognitive architecture [16fc7d2](https://github.com/JerkyTreats/meld/commit/16fc7d219cfc6ed3f1abb377c27ff6d765f8f483)

### Chores

* **format:** apply rustfmt to workspace publish files [cf7a04b](https://github.com/JerkyTreats/meld/commit/cf7a04ba740bbe2208c33b3db6c80117112ed7db)


## [2.3.1](https://github.com/JerkyTreats/meld/compare/v2.3.0...v2.3.1) — 2026-04-06

### Bug Fixes

* **workflows:** add configurable provider retry for failed model responses [9cb745c](https://github.com/JerkyTreats/meld/commit/9cb745ce1b20e068e8e873c3df357ad4968ab712)


## [2.3.0](https://github.com/JerkyTreats/meld/compare/v2.2.5...v2.3.0) — 2026-04-05

### Features

* **provider:** support additional_json overrides in completion payloads [6786b68](https://github.com/JerkyTreats/meld/commit/6786b68867f617c4c897208806cfbef83c96f224)
* **workflow:** externalize docs writer workflow assets for init [39c0736](https://github.com/JerkyTreats/meld/commit/39c0736734e96eff5c87567a518c02d72aa23463)
* **capability:** add shared contract and catalog core [5c901f2](https://github.com/JerkyTreats/meld/commit/5c901f294872041a00e1530d15cba25ff80f8106)
* **task:** add records artifact repo and compiler [2782cf8](https://github.com/JerkyTreats/meld/commit/2782cf8025e4056952ec14a435929bd9ae10bc9a)
* **task:** add executor readiness and payload assembly [05f2dec](https://github.com/JerkyTreats/meld/commit/05f2decbef37b118df27010756459f7c2ba0cc02)
* **capability:** publish first slice domain invokers [927a7aa](https://github.com/JerkyTreats/meld/commit/927a7aa094fec5fd15a01fe64d427996e9158b78)
* **task:** run docs writer as compiled task package [32a7874](https://github.com/JerkyTreats/meld/commit/32a78749803d9ae1143f7facc669f0121fb53092)
* **task:** add artifact-driven task expansion [86c4ed2](https://github.com/JerkyTreats/meld/commit/86c4ed26ec70fdea241a70627d53128240f38681)
* **task:** load workflow packages from external specs [522e5da](https://github.com/JerkyTreats/meld/commit/522e5dafdaa414b0eae32c819badad6802ba43e8)

### Bug Fixes

* **repo:** tolerate wrapped docs writer JSON and ignore eval-only CI [1f30bd9](https://github.com/JerkyTreats/meld/commit/1f30bd944467089d60702ec11973f8d3183f9954)
* **ci:** ignore bad commit types [7d25435](https://github.com/JerkyTreats/meld/commit/7d254355cd36c0e8fed82c17e737c640eb2c5047)

### Refactors

* **provider:** unify runtime request overrides [6da2c6e](https://github.com/JerkyTreats/meld/commit/6da2c6edfb38b16ba691255c8c8ee4c78b74022f)
* **context:** carry provider bindings through generate requests [f131363](https://github.com/JerkyTreats/meld/commit/f131363149b1ebe46154a21d53a3fb812d5fa361)
* **workflow:** load workflows from XDG only [10edc24](https://github.com/JerkyTreats/meld/commit/10edc2499a72e67ff96eff042ea24c0690bcc73e)
* **merkle_traversal:** extract structural traversal batches [b022e31](https://github.com/JerkyTreats/meld/commit/b022e31a647cfe09ab6d37570dd212373a1ed952)
* **control:** move generation orchestration under control [d0e8f67](https://github.com/JerkyTreats/meld/commit/d0e8f6746184d61fe974c6a8c1c8725f9b28ff90)
* **provider:** move generation transport into provider executor [7e6f67e](https://github.com/JerkyTreats/meld/commit/7e6f67ec40e98443e7ba52ed5f2f047d93953239)
* **workflow:** route queue dispatch through control compatibility [4f581bb](https://github.com/JerkyTreats/meld/commit/4f581bb69895e75280510ecc1424a2b6bd79689d)
* **workflow:** route docs writer through task runtime [321775f](https://github.com/JerkyTreats/meld/commit/321775f358e725aa79290d80b8750a83719b51b4)

### Tests

* **progress:** characterize bottom-up recursive generate ordering [ad7e2fd](https://github.com/JerkyTreats/meld/commit/ad7e2fdc409c2c3d5b4a9a5aea39839a2de6e26d)
* **workflow:** add task path compatibility coverage [40d18be](https://github.com/JerkyTreats/meld/commit/40d18be7fed42e2d7e434d0787462c89ab70e814)

### CI

* **release:** skip runs without release-relevant changes [3fe9201](https://github.com/JerkyTreats/meld/commit/3fe9201fd19f5af1f0cdd2fc8a136dd35ef6a802)

### Chores

* **eval:** isolate local override config and default web-search off [30a8991](https://github.com/JerkyTreats/meld/commit/30a89911abfb9369d9ac83b3b667053839d7bd4f)
* **lint:** fix fmt and clippy violations [97f0fff](https://github.com/JerkyTreats/meld/commit/97f0fff3abed38d9f0fe0d13d8080f7ef4296865)

### Design

* **capabilities:** archive refactor docs and task plans [75e6cc7](https://github.com/JerkyTreats/meld/commit/75e6cc7046dfbbaad6d299095ff5395b9b0a2858)
* **capabilities:** remove archived refactor stubs [7d13b87](https://github.com/JerkyTreats/meld/commit/7d13b877bbbfd283956c21722bd75553654234cd)
* **control:** define event manager requirements and telemetry refactor [ee600b6](https://github.com/JerkyTreats/meld/commit/ee600b6b7b186d1c0aa3d33fad7dcb764fdd7a39)

### Policy

* **governance:** lock capability task baseline gates [4c63da0](https://github.com/JerkyTreats/meld/commit/4c63da028659bd8d564a3064ff2aabeb5c5287fa)
* **agents:** reference commenting guidance in agent rules [7861a68](https://github.com/JerkyTreats/meld/commit/7861a68336dbb04450f42aeabfb91c03046f4a23)


## [2.2.5](https://github.com/JerkyTreats/meld/compare/v2.2.4...v2.2.5) — 2026-03-27

### Design

* **workflow_orchestrator:** refresh design overview screenshot [8b28e29](https://github.com/JerkyTreats/meld/commit/8b28e297c1bbbd9fdb391ef1e17eeb6b05c3220f)


## [2.2.4](https://github.com/JerkyTreats/meld/compare/v2.2.3...v2.2.4) — 2026-03-27

### Design

* **workflow_orchestrator:** replace workflow docs with capability and plan model [3209b2d](https://github.com/JerkyTreats/meld/commit/3209b2d85b581bcd5c8b717a74de4396e9a9ce0d)


## [2.2.3](https://github.com/JerkyTreats/meld/compare/v2.2.2...v2.2.3) — 2026-03-20

### Design

* **workflow_orchestrator:** streamline roadmap read order [ccf7519](https://github.com/JerkyTreats/meld/commit/ccf7519c6cb1e597a3c7358f3c6146051463381b)


## [2.2.2](https://github.com/JerkyTreats/meld/compare/v2.2.1...v2.2.2) — 2026-03-14

### Refactors

* **telemetry:** move summary mapping into domain modules [ce1c6e8](https://github.com/JerkyTreats/meld/commit/ce1c6e8f6f0e1aedd32d418c58afcf9bb7324b0b)

### Documentation

* **htn:** add research corpus and reports [cf78595](https://github.com/JerkyTreats/meld/commit/cf78595af9eb2c11e687ed77f8b1c1d8e6b8ba8c)

### Design

* **workflow_orchestrator:** add context refactor design workstream [b1ddfb1](https://github.com/JerkyTreats/meld/commit/b1ddfb1a5a8f2ffa4d632b4786ebeb68b6146a3f)


## [2.2.1](https://github.com/JerkyTreats/meld/compare/v2.2.0...v2.2.1) — 2026-03-13

### Refactors

* **workflow:** normalize docs writer gates and reorganize workflow design [152f59c](https://github.com/JerkyTreats/meld/commit/152f59c6c9664169d063dbcfdaa459c5df8952f8)

### Design

* **workflow_orchestrator:** align orchestration docs with HTN foundation [ea29e8d](https://github.com/JerkyTreats/meld/commit/ea29e8dadb72cf51f731d3e3d4f9d5585950767e)
* **workflow:** shift orchestrator design to task model [89f84cf](https://github.com/JerkyTreats/meld/commit/89f84cfc80634267735428424dce170afcba692a)


## [2.2.0](https://github.com/JerkyTreats/meld/compare/v2.1.0...v2.2.0) — 2026-03-09

### Features

* **workspace:** add danger flush for runtime state reset [023b4ab](https://github.com/JerkyTreats/meld/commit/023b4abd2168ffa555a81b4aa39e93c3d0a4e59e)

### Bug Fixes

* **context:** start generation timeout at worker pickup [caa87eb](https://github.com/JerkyTreats/meld/commit/caa87eb5833daa748e84767438b2d3b88f77c53e)
* **workspace:** treat stale scans as usable state [428fbff](https://github.com/JerkyTreats/meld/commit/428fbffe1823ac18a8a84d7d20b8022131cc01a6)

### CI

* **release:** automate changelog updates and backfill releases [e2a8198](https://github.com/JerkyTreats/meld/commit/e2a8198e88ea9a868d90ab29f1f89c44dd5821b0)

### Design

* **tui:** align specs with current meld architecture [460ed4e](https://github.com/JerkyTreats/meld/commit/460ed4eb232e1e041a597a407c56526147819077)


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
