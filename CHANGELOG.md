# Changelog

## [0.6.1](https://github.com/baprx/zed-yaml-multi-schema/compare/v0.6.0...v0.6.1) (2026-08-28)


### Bug Fixes

* **deps:** update rust crate jsonschema to 0.50 ([3d43c29](https://github.com/baprx/zed-yaml-multi-schema/commit/3d43c295fb10d23cf344dbd12d0649ab841615fb))
* **deps:** update rust crate ureq to v3 ([24f3a0e](https://github.com/baprx/zed-yaml-multi-schema/commit/24f3a0eeb52a005013ca33a4e49829dfce12e4ac))
* **lsp:** adapt remote fetch to ureq 3 body API ([433064b](https://github.com/baprx/zed-yaml-multi-schema/commit/433064b07f853b889b64439543dd31867ae85dd0))
* **validator:** adapt to jsonschema 0.50 instance_path API change ([eae8dc6](https://github.com/baprx/zed-yaml-multi-schema/commit/eae8dc60e818a7bd5d6798070608a12138a6c6e9))

## [0.6.0](https://github.com/baprx/zed-yaml-multi-schema/compare/v0.4.0...v0.6.0) (2026-08-28)


### Features

* auto-install LSP binary and restore dev extension build ([8ae4dde](https://github.com/baprx/zed-yaml-multi-schema/commit/8ae4dde3245d2387ffc3da25b5f0fe3ded18fd21))

## [0.4.0](https://github.com/baprx/zed-yaml-multi-schema/compare/v0.3.0...v0.4.0) (2026-08-27)


### Features

* auto-trigger completions while typing YAML keys ([3989203](https://github.com/baprx/zed-yaml-multi-schema/commit/398920376a8d8952f54a03cba4255bc8656f820e))

## [0.3.0](https://github.com/baprx/zed-yaml-multi-schema/compare/v0.2.2...v0.3.0) (2026-08-27)


### Features

* auto-trigger completions while typing YAML keys ([4e536b4](https://github.com/baprx/zed-yaml-multi-schema/commit/4e536b4777335f17d3564091a44f9cc5bb8dfbbb))

## [0.2.2](https://github.com/baprx/zed-yaml-multi-schema/compare/v0.2.1...v0.2.2) (2026-08-26)


### Features

* filter already-present keys from completions to avoid duplicates ([6f427f5](https://github.com/baprx/zed-yaml-multi-schema/commit/6f427f5c9e5d25508b68f0c98da77a97392e6b6c))

## [0.2.1](https://github.com/baprx/zed-yaml-multi-schema/compare/v0.2.0...v0.2.1) (2026-08-26)


### Bug Fixes

* underline full diagnostic line and drop trailing blank line from structure snippet ([da15754](https://github.com/baprx/zed-yaml-multi-schema/commit/da15754368b6e77a5d24950249a23774d1d9ef85))

## [0.2.0](https://github.com/baprx/zed-yaml-multi-schema/compare/v0.1.1...v0.2.0) (2026-08-26)


### Features

* seed object structure via snippets, line-level diagnostics, and fix completion after sibling keys ([f305d86](https://github.com/baprx/zed-yaml-multi-schema/commit/f305d866d503facc7c6d82d8850891f5a9a9dee2))


### Bug Fixes

* emit valid LSP messages for diagnostics and completion kinds ([d8b9cdf](https://github.com/baprx/zed-yaml-multi-schema/commit/d8b9cdf0bba3a50f9e5b9431ae80334f91862d60))
* include jsonrpc 2.0 in LSP messages so Zed can deserialize them ([434b0f1](https://github.com/baprx/zed-yaml-multi-schema/commit/434b0f182bb3aef65b2715df30665d7ae58d6fc5))
* revalidate on textDocument/didChange so type errors appear while editing ([76ee698](https://github.com/baprx/zed-yaml-multi-schema/commit/76ee69818617d5540bbceb0a5789fff60a41f1f1))

## [0.1.1](https://github.com/baprx/zed-yaml-multi-schema/compare/v0.1.0...v0.1.1) (2026-08-26)


### Bug Fixes

* compile Zed extension for wasm32-wasip2 ([0090913](https://github.com/baprx/zed-yaml-multi-schema/commit/0090913bef34eca77eca556cbf68fbcf8f6963bb))
* retry failed schema references on change and add SC-001 coverage ([7097032](https://github.com/baprx/zed-yaml-multi-schema/commit/7097032aa5f769b89dee6cbe88bbf0c59743fe15))

## 0.1.0 (2026-08-26)


### Features

* add multi-schema YAML validation for Helm umbrella charts ([abc0973](https://github.com/baprx/zed-yaml-multi-schema/commit/abc09737d067b6c78c8e20442fa16ebbc67c6204))
