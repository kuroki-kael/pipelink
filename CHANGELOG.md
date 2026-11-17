# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]
### Features

- **events:** Add graph snapshot, subscription, and installed app queries ([`23d9492`](https://codeberg.org/kuroki-kael/pipelink/commit/23d9492525e7ac5e30e8f7a43a11ff9379f576ce))
- **ipc:** Extract shared Unix socket IPC into pipelink-ipc ([`54069be`](https://codeberg.org/kuroki-kael/pipelink/commit/54069bec2a816c39c8a1af6bb5d8ba847934794f))
- **default-sink:** Track live sink updates in snapshots ([`6e86a9d`](https://codeberg.org/kuroki-kael/pipelink/commit/6e86a9d42230f65c8d46beb39afe16b1824aef03))
- **assets:** Add app icons and shared UI vector assets ([`563e765`](https://codeberg.org/kuroki-kael/pipelink/commit/563e7654234c48721f51adbfa3f490079a8f5a06))
- **gui:** Add WaveLink-inspired iced desktop interface ([`1ca7827`](https://codeberg.org/kuroki-kael/pipelink/commit/1ca7827273c2b4bdba572bc4cb9e5b8b651bc48b))
- **gui:** Add channel detail modal and app routing picker ([`41d58d6`](https://codeberg.org/kuroki-kael/pipelink/commit/41d58d60b6d095ef7cbd8b026a1a0d7f9cd84152))
- **packaging:** Add desktop entry and pipelink-gui builds ([`44f3111`](https://codeberg.org/kuroki-kael/pipelink/commit/44f3111691a700b82ea7fc6a798bbccae8580c16))
- **common:** Add master volume, reorder, and rename types ([`43207ce`](https://codeberg.org/kuroki-kael/pipelink/commit/43207ce7019a9e2f7899d6f4a16aa909a7396cd0))
- **pipewire:** Support null-sink master volume and mute via Props ([`05f6a3e`](https://codeberg.org/kuroki-kael/pipelink/commit/05f6a3e9ac1699dd7dc63bb9d7c36fc37ac98a1e))
- **jack:** Support real-time channel and mix reordering ([`e2eff58`](https://codeberg.org/kuroki-kael/pipelink/commit/e2eff583b2275d14ac14ca137e52e68d05c46afb))
- **engine:** Add master volume, reorder, and clean app unrouting ([`14d4404`](https://codeberg.org/kuroki-kael/pipelink/commit/14d4404d9f4c1347af77ccce931acb36e8e77dad))
- **engine:** Implement make-before-break channel and mix renaming ([`9500b76`](https://codeberg.org/kuroki-kael/pipelink/commit/9500b76659386f000429e11b6c3f9f2ca694e81a))
- **cli:** Add channel and mix rename subcommands ([`244f632`](https://codeberg.org/kuroki-kael/pipelink/commit/244f632dbebf50cfb97330e1bfb7afc5652ed5a3))
- **gui:** Add channel master controls, reorder, rename, and app picker ([`4443932`](https://codeberg.org/kuroki-kael/pipelink/commit/444393237e055dab12d9df03855278bf91d57bc5))
- **gui:** Enforce single window instance via runtime socket ([`d065b76`](https://codeberg.org/kuroki-kael/pipelink/commit/d065b76161a1719b10d4da2f7a4eb626d57616c0))
- **tray:** Add daemon StatusNotifierItem icon with detached GUI launch ([`f3504b7`](https://codeberg.org/kuroki-kael/pipelink/commit/f3504b79cab6dac3154dd591b3c284b06203494a))
- **gui:** Replace mute text buttons with speaker icons ([`e795f56`](https://codeberg.org/kuroki-kael/pipelink/commit/e795f563e95929ac970b3d2794b089f622bf9470))

### Refactoring

- **events:** Unify channel and mix failure state events ([`7ceb6b7`](https://codeberg.org/kuroki-kael/pipelink/commit/7ceb6b7345045737615ac562cdc9a20c79b919dc))

### Styling

- **gui:** Align matrix cell heights dynamically to channel rows ([`3c08bbf`](https://codeberg.org/kuroki-kael/pipelink/commit/3c08bbf44fc05c85dad4540362813e3ad26fda5f))

### Build, CI & Maintenance

- **release:** Switch release changelog generation to git-cliff ([`92ae4e7`](https://codeberg.org/kuroki-kael/pipelink/commit/92ae4e73c7523a3b36f008c36555631db30edb4a))

## [0.1.1] - 2026-11-16
### Features

- **common:** Add categorized logger with journald support ([`ce57fe8`](https://codeberg.org/kuroki-kael/pipelink/commit/ce57fe85ee3e66bd0116f2b79a6b22be53a378aa))

### Documentation

- Add README with installation, daemon setup, CLI, and config reference ([`770345e`](https://codeberg.org/kuroki-kael/pipelink/commit/770345e50c3aa129aa26d1d52b2a4ba219706ea8))
- Note Codeberg origin and GitHub mirror in README ([`3b0e25c`](https://codeberg.org/kuroki-kael/pipelink/commit/3b0e25cc63df3def6b1a88bb5f2e70d1c8bbb028))
- Add AGENTS.md for agent rules discovery ([`47d523b`](https://codeberg.org/kuroki-kael/pipelink/commit/47d523b217104406f3e325d9e536dc5713ad265b))

### Build, CI & Maintenance

- **cargo:** Prepare workspace crate metadata for registry publishing ([`3a100c1`](https://codeberg.org/kuroki-kael/pipelink/commit/3a100c13405d8801952d50f06c788fb758e39d8f))
- **release:** Publish Debian, Arch, and Cargo packages on main-branch tags ([`78bbbb5`](https://codeberg.org/kuroki-kael/pipelink/commit/78bbbb52ce2ddfe405832d7cdefe9003fa215406))
- **release:** Fix Codeberg Cargo auth header and add GitHub release mirror ([`384f4fc`](https://codeberg.org/kuroki-kael/pipelink/commit/384f4fcd283d98a97bdb4a69073ea0febf6e6a7e))
- **release:** Generate automatic release changelog from commits ([`50a46a2`](https://codeberg.org/kuroki-kael/pipelink/commit/50a46a242f9b3aebdb55e68190bc1243508e8886))


**Full Changelog:** https://codeberg.org/kuroki-kael/pipelink/compare/v0.1.0...v0.1.1
## [0.1.0] - 2026-11-16
### Features

- **common:** Add shared models, events, and graph types ([`dcdde9c`](https://codeberg.org/kuroki-kael/pipelink/commit/dcdde9c70e7f923288790afc50c93b697302f10f))
- **pipewire:** Implement loop and module management ([`14d5644`](https://codeberg.org/kuroki-kael/pipelink/commit/14d564468056091d9b27dce9f6b912ecc9c4231d))
- **jack:** Add client wrapper and port management ([`2d3dec5`](https://codeberg.org/kuroki-kael/pipelink/commit/2d3dec546542f99b73366e8aea32d4f35903d0a5))
- **resolver:** Implement process and desktop resolver ([`8e259e4`](https://codeberg.org/kuroki-kael/pipelink/commit/8e259e489cb30474c280f889604b279dfc693c9f))
- **engine:** Add topology manager and executor ([`ccce98d`](https://codeberg.org/kuroki-kael/pipelink/commit/ccce98d44efce0cf67bdbb43cfe39f01cce62f25))
- **app:** Add cli interface and daemon runner ([`407c919`](https://codeberg.org/kuroki-kael/pipelink/commit/407c9198053178a4f7426f9a50bab1b4e02cdc88))
- **common:** Introduce strongly typed domain primitives ([`a3f3c0f`](https://codeberg.org/kuroki-kael/pipelink/commit/a3f3c0f6f2a721c822075d107c03dd0eab9eee8a))
- **app:** Add daemon subcommand ([`c1a26cb`](https://codeberg.org/kuroki-kael/pipelink/commit/c1a26cbd1987d9e7e3db4e9acf04d129e988da4f))
- **pipewire:** Create mixes through the loopback module ([`8838e00`](https://codeberg.org/kuroki-kael/pipelink/commit/8838e00b45035027580adf39b0d124e9139cdd93))
- **engine:** Track mixes in the graph ([`351ad74`](https://codeberg.org/kuroki-kael/pipelink/commit/351ad74a269e31e4a4ef15db9b313be16b514d70))
- **engine:** Answer graph queries over the control envelope ([`a8be0fa`](https://codeberg.org/kuroki-kael/pipelink/commit/a8be0fa31f396ee5f824c51c341a4933cf7ece51))
- **app:** Speak json lines over the control socket ([`543e669`](https://codeberg.org/kuroki-kael/pipelink/commit/543e669f2efcfa73b4e3ac37a6563847b9f81630))
- **app:** Drive the daemon from the cli ([`69b00bc`](https://codeberg.org/kuroki-kael/pipelink/commit/69b00bc9e5671110b63c5fa380294fb027e75775))
- **engine:** Report a removal that could not be done ([`060029e`](https://codeberg.org/kuroki-kael/pipelink/commit/060029e8b308c036c2540db62931dc552cd4dbf5))
- **app:** Report a failed removal in the cli ([`83957b6`](https://codeberg.org/kuroki-kael/pipelink/commit/83957b68c2d17e33b6247bf7c9aa6d50e43f0f09))
- **engine:** Map a failed destruction to a failed removal ([`4fd1f68`](https://codeberg.org/kuroki-kael/pipelink/commit/4fd1f68c6f0011f66bf1d2a8ffd7672f696f0e28))
- **common:** Add N:M channel-mix routing matrix to AudioGraph ([`5e0804e`](https://codeberg.org/kuroki-kael/pipelink/commit/5e0804efcc6ef4b386a5dd67f1328839af62d117))
- **common:** Expose JACK client name and routing matrix helpers ([`eadc800`](https://codeberg.org/kuroki-kael/pipelink/commit/eadc8007c07167d481d1b58413dd1904579078ef))
- **common:** Switch channel gain events from dB to percentage ([`876ae6c`](https://codeberg.org/kuroki-kael/pipelink/commit/876ae6c4f3c6784fd228435f4559ac820a803888))
- **jack:** Implement real-time JACK mixer client and process handler ([`f563f06`](https://codeberg.org/kuroki-kael/pipelink/commit/f563f06c2282f885b56bfd9ceca2f4bab4598dde))
- **engine:** Wire JackClient into executor control and PipeWire events ([`057e1ec`](https://codeberg.org/kuroki-kael/pipelink/commit/057e1ecb819749f52a9ce20797b8829d3aab8b41))
- **common:** Track JACK ports and PipeWire stereo links on Channel and Mix ([`851e2df`](https://codeberg.org/kuroki-kael/pipelink/commit/851e2df32387cff92940118124ad88b583221eea))
- **pipewire:** Discover port names and handle PipeWire link lifecycle ([`fb0d669`](https://codeberg.org/kuroki-kael/pipelink/commit/fb0d6691655ac734bc60a4f8e9824bed487909a0))
- **engine:** Auto-link channels and mixes to PipeLink JACK ports ([`f20bb10`](https://codeberg.org/kuroki-kael/pipelink/commit/f20bb10808dfa7262fb5fdcd712aecebbe4a6145))
- **common:** Add RouteOperationFailed state event ([`7cb8e7e`](https://codeberg.org/kuroki-kael/pipelink/commit/7cb8e7eb0861fca9101eef114f0c0c55b1378e81))
- **engine:** Complete route, gain, mute, and mix-channel query handling ([`aac96d6`](https://codeberg.org/kuroki-kael/pipelink/commit/aac96d69bdea9d5800bcdf286ca6d1766436327f))
- **cli:** Implement route list, set, remove, gain, mute, and unmute ([`06d46b3`](https://codeberg.org/kuroki-kael/pipelink/commit/06d46b3b7814798b4c64ce23547ba8e793785714))
- **common:** Track multiple stream nodes per AppNode and add app routing events ([`0765df2`](https://codeberg.org/kuroki-kael/pipelink/commit/0765df2881c2252c564dfac60d4479eadb615033))
- **engine:** Implement ConfigGraph-backed app-to-channel routing ([`2949915`](https://codeberg.org/kuroki-kael/pipelink/commit/2949915096fffe6a5e24c61b44119fcacc58d9f8))
- **cli:** Add app routing subcommand and show app routes in status ([`08ee38b`](https://codeberg.org/kuroki-kael/pipelink/commit/08ee38b07e154543dc5d7a537553b3a7b863aa27))
- **common:** Add output device model, output events, and case-insensitive graph lookups ([`d8ae93b`](https://codeberg.org/kuroki-kael/pipelink/commit/d8ae93bd03e857c4d6d6a4c160d2b1582fadabb2))
- **pipewire:** Expose node.description and hardware device flag on NodeAppeared ([`d1b7d9b`](https://codeberg.org/kuroki-kael/pipelink/commit/d1b7d9b01d7c462e2d27d1cdf388fe4ca0d2cbdc))
- **engine:** Implement active mix and stereo output routing with auto-failover ([`114080a`](https://codeberg.org/kuroki-kael/pipelink/commit/114080a0af09f8c242b233663dd51ee797c139fb))
- **cli:** Add output commands, mix active switching, and case-insensitive matching ([`e77afa6`](https://codeberg.org/kuroki-kael/pipelink/commit/e77afa6ea6add2f73903c8f57157f6b5cdb14cab))
- **config:** Add XDG config/state persistence and remove source from AudioChannel ([`1a2e26a`](https://codeberg.org/kuroki-kael/pipelink/commit/1a2e26af14e7130367fb70ac663843783358a0ba))
- **engine:** Restore state on bootstrap, persist config/state, and preserve config on unexpected removals ([`e2d54b1`](https://codeberg.org/kuroki-kael/pipelink/commit/e2d54b1ca6a16d6b47ba8735b38d04ba081a43b7))
- **daemon:** Load config/state on startup and handle SIGINT/SIGTERM shutdown ([`b4453ac`](https://codeberg.org/kuroki-kael/pipelink/commit/b4453ace3583e6e92ea1887a4c1603b0b9347550))
- **config:** Initialize starter topology on first run and split config modules ([`22939bc`](https://codeberg.org/kuroki-kael/pipelink/commit/22939bc3c341d64c110308c2d601f177cf7437a9))
- **default-sink:** Add WirePlumber default sink command and refactor oversized modules ([`eb04e9c`](https://codeberg.org/kuroki-kael/pipelink/commit/eb04e9ce03834d26b78c192df292387eab1cdc01))

### Bug Fixes

- **engine:** Report the real result of a channel or mix creation ([`95749ff`](https://codeberg.org/kuroki-kael/pipelink/commit/95749ff07bd1e8a7908954e41b5bd2772a449f64))
- **pipewire:** Correct the mix node description ([`d3126b5`](https://codeberg.org/kuroki-kael/pipelink/commit/d3126b5192fca4d7e64a1463f76218e643b3ae73))
- **pipewire:** Report a destruction that did not work ([`3a44cf6`](https://codeberg.org/kuroki-kael/pipelink/commit/3a44cf612d28dc8b39bc40e03309d4a4567dd711))
- **jack:** Track pending retired port reclaims on JackClient ([`2882eb1`](https://codeberg.org/kuroki-kael/pipelink/commit/2882eb1eb62a58252bd753e551e6f87b5f445690))
- **engine:** Poll JackClient::reclaim while retired ports are pending ([`9ab6d38`](https://codeberg.org/kuroki-kael/pipelink/commit/9ab6d38e975ba737c6d2d90269fd37887db6c8a8))
- **resolver:** Fix double-quote closing and POSIX escaping in tokenize_cmdline ([`31f088c`](https://codeberg.org/kuroki-kael/pipelink/commit/31f088caf8989d14665adf373b99e9bdb03dc61b))
- **resolver:** Only classify process itself as Electron or Chromium ([`d9e53a5`](https://codeberg.org/kuroki-kael/pipelink/commit/d9e53a52cdbc0218bbf70c59de149a8a080f7020))
- **ipc:** Isolate control socket in user-owned directory with cleanup guard ([`8dd113f`](https://codeberg.org/kuroki-kael/pipelink/commit/8dd113f25f60b635e49895dd615ba669cfd838b5))
- **pipewire/engine:** Use unbounded channel for PipeWire-to-engine events ([`4b162ad`](https://codeberg.org/kuroki-kael/pipelink/commit/4b162adc1d8f97bd7445616dc3ce43b5b85bee5c))

### Refactoring

- **common:** Migrate graph and event models to strong types ([`c70b950`](https://codeberg.org/kuroki-kael/pipelink/commit/c70b95064c837164e504cc4ef574c57f0ad3c61c))
- **resolver:** Use typed domain identifiers ([`a9c71bd`](https://codeberg.org/kuroki-kael/pipelink/commit/a9c71bdbd53016b6b9505b10c1faf634879a98f6))
- **pipewire:** Type identifiers and split event module ([`9227210`](https://codeberg.org/kuroki-kael/pipelink/commit/9227210dce7676e296d4061f6edeb1cabdc0160c))
- **engine:** Use typed domain ids in topology and executor ([`7f8bda4`](https://codeberg.org/kuroki-kael/pipelink/commit/7f8bda43ff06abccd2b961e82b6263d6a2b0c9c3))
- **common:** Restrict node helpers to crate scope ([`c971549`](https://codeberg.org/kuroki-kael/pipelink/commit/c97154914e84a484a709270e84d001ff8518167e))
- **pipewire:** Replace glob re-exports with explicit imports ([`cfdf212`](https://codeberg.org/kuroki-kael/pipelink/commit/cfdf2121c207f05805d1eb506bee979c8f898a75))
- **resolver:** Restrict process helpers to crate scope ([`a508fdc`](https://codeberg.org/kuroki-kael/pipelink/commit/a508fdc76733dcd8045d63aad94767d40ea2c99f))
- **engine:** Simplify config topology ([`c916d06`](https://codeberg.org/kuroki-kael/pipelink/commit/c916d064d36fb31c92108f7fc1aad9928ce05bdb))
- **engine:** Split the event handlers into submodules ([`6d96edf`](https://codeberg.org/kuroki-kael/pipelink/commit/6d96edf88b003a11d88ad13c75a4646919e676cd))
- **app:** Rename the package to pipelink ([`eed19c9`](https://codeberg.org/kuroki-kael/pipelink/commit/eed19c920efd7d4268337f642ac7d4e67a1e694c))
- **common:** Split the domain events into submodules ([`f89b8b9`](https://codeberg.org/kuroki-kael/pipelink/commit/f89b8b9f4c863a08b78215944a9f5787f3796513))
- **engine:** Migrate executor to common AudioGraph ([`c5299c7`](https://codeberg.org/kuroki-kael/pipelink/commit/c5299c7c95a07bd92537e96c9b001fdf56d8e682))
- **common:** Consolidate port types and MixerPort naming in graph::port ([`bb6f8f2`](https://codeberg.org/kuroki-kael/pipelink/commit/bb6f8f2eaf0578080f011aaee93ed1baa53c794a))
- **jack/engine:** Sync pre-built gains matrix via rtrb, share MixerPort, and clamp gain at 400% ([`4509744`](https://codeberg.org/kuroki-kael/pipelink/commit/450974448a9bf1970a6ce36244ead832c0e36671))
- **pipewire:** Pass lightweight target fields in Command::SetAppRoute ([`73c8c9a`](https://codeberg.org/kuroki-kael/pipelink/commit/73c8c9a6535155b4b4310ce5ff753565ebda8deb))

### Documentation

- **workspace:** Add missing rustdoc comments ([`5dc207c`](https://codeberg.org/kuroki-kael/pipelink/commit/5dc207c590b9d6f8a8d2c80f496091c4a971bba6))

### Styling

- **workspace:** Format code with nightly rustfmt ([`8807608`](https://codeberg.org/kuroki-kael/pipelink/commit/8807608b4088c5ffd0900be85a16865d47e752be))
- **common:** Format audio graph ([`69e3b85`](https://codeberg.org/kuroki-kael/pipelink/commit/69e3b859e6044a9bb9c593c6ff8f2d24c4ae4d3c))

### Testing

- **common:** Pin event wire format ([`6d7371b`](https://codeberg.org/kuroki-kael/pipelink/commit/6d7371bd21dea44ec7d3c7407a67d6149a344282))
- **app:** Exercise mix creation in the daemon smoke test ([`6afecad`](https://codeberg.org/kuroki-kael/pipelink/commit/6afecad93d1ec815c60e287b59ec756c7a6cff15))

### Build, CI & Maintenance

- **workspace:** Initialize project workspace and configs ([`edbc6e9`](https://codeberg.org/kuroki-kael/pipelink/commit/edbc6e90ee70a2d31df07ec31cfb3ec42aa8d03a))
- **ci:** Add forgejo actions workflow for prs to main ([`6329f2e`](https://codeberg.org/kuroki-kael/pipelink/commit/6329f2e262e697e62af2a53bf4d39f11987db213))
- **workspace:** Standardize dependency declarations ([`bfe5e82`](https://codeberg.org/kuroki-kael/pipelink/commit/bfe5e82fad54001f7a18f1b50280fdc1191d9a4a))
- **app:** Declare web and midi feature flags ([`2dc49cb`](https://codeberg.org/kuroki-kael/pipelink/commit/2dc49cb03c423b1cb2f782d92081f9b4496bc4ad))
- **ci:** Check dev and feature branches ([`07f7512`](https://codeberg.org/kuroki-kael/pipelink/commit/07f75128fdfb78be21f7cba62d9ee75bbd017de0))
- **ci:** Install clippy on the runner ([`4db8dd4`](https://codeberg.org/kuroki-kael/pipelink/commit/4db8dd44e43a7cda6f6092dc4bda2fdcde7fe9a6))
- **jack:** Update the jack crate to 0.14 ([`1ecc405`](https://codeberg.org/kuroki-kael/pipelink/commit/1ecc405dce37efebf94d1f678c421eec2a8f1cf7))
- **packaging:** Add Arch PKGBUILD, Debian/Makefile packaging, systemd unit, and CI workflow ([`83f727e`](https://codeberg.org/kuroki-kael/pipelink/commit/83f727e6460cbdce24e28e88a4c4175336ec63f4))

