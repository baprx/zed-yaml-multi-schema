# Changelog

## [0.5.0](https://github.com/baprx/zed-yaml-multi-schema/compare/v0.4.0...v0.5.0) (2026-08-28)


### Features

* add multi-schema YAML validation for Helm umbrella charts ([43641e1](https://github.com/baprx/zed-yaml-multi-schema/commit/43641e168f587515876baa031365eba494f0f79d))
* auto-install LSP binary and restore dev extension build ([aa9b333](https://github.com/baprx/zed-yaml-multi-schema/commit/aa9b33396f6681fc674b34d8c88d44bc46853c79))
* auto-trigger completions while typing YAML keys ([04ffbf0](https://github.com/baprx/zed-yaml-multi-schema/commit/04ffbf00c19274b1fbb8675c14ac564d39b96a34))
* auto-trigger completions while typing YAML keys ([aa07523](https://github.com/baprx/zed-yaml-multi-schema/commit/aa07523f81bb686ac249b7b89a12ec58c5cdc11f))
* filter already-present keys from completions to avoid duplicates ([055f924](https://github.com/baprx/zed-yaml-multi-schema/commit/055f9245c6a94342267935fe4d8d9579489f7be3))
* seed object structure via snippets, line-level diagnostics, and fix completion after sibling keys ([9659cfa](https://github.com/baprx/zed-yaml-multi-schema/commit/9659cfa4dfb9a1f40bad33de529737e27c00f6ad))


### Bug Fixes

* compile Zed extension for wasm32-wasip2 ([115f6f7](https://github.com/baprx/zed-yaml-multi-schema/commit/115f6f7eb16e8d833648dff52d06fa2398ac38fb))
* emit valid LSP messages for diagnostics and completion kinds ([32b1c7d](https://github.com/baprx/zed-yaml-multi-schema/commit/32b1c7dae99bdb634375a06771b3d0415e617af9))
* include jsonrpc 2.0 in LSP messages so Zed can deserialize them ([357b714](https://github.com/baprx/zed-yaml-multi-schema/commit/357b7145fd4dc4bec2b9417efb131905e05cae12))
* retry failed schema references on change and add SC-001 coverage ([3d37bf4](https://github.com/baprx/zed-yaml-multi-schema/commit/3d37bf4bcc54bc0fe7a661740cc63364290cb597))
* revalidate on textDocument/didChange so type errors appear while editing ([8982ea0](https://github.com/baprx/zed-yaml-multi-schema/commit/8982ea09a8b0df990d3864badc0acb50e2a58279))
* underline full diagnostic line and drop trailing blank line from structure snippet ([a8458a4](https://github.com/baprx/zed-yaml-multi-schema/commit/a8458a47c979a4fa0c6babcd843efc9ee0df35fd))

## [0.4.0](https://github.com/baprx/zed-yaml-multi-schema/compare/v0.3.0...v0.4.0) (2026-08-27)


### Features

* auto-trigger completions while typing YAML keys ([d3e053e](https://github.com/baprx/zed-yaml-multi-schema/commit/d3e053e80c221679a5fb9d7948fb1dcbc6a72199))

## [0.3.0](https://github.com/baprx/zed-yaml-multi-schema/compare/v0.2.2...v0.3.0) (2026-08-27)


### Features

* auto-trigger completions while typing YAML keys ([7a68336](https://github.com/baprx/zed-yaml-multi-schema/commit/7a68336222b1715cf0e72e44cb15d561322fb6be))

## [0.2.2](https://github.com/baprx/zed-yaml-multi-schema/compare/v0.2.1...v0.2.2) (2026-08-26)


### Features

* filter already-present keys from completions to avoid duplicates ([595ed1c](https://github.com/baprx/zed-yaml-multi-schema/commit/595ed1c52812cae163df8da3a3f1b656dfe3877f))

## [0.2.1](https://github.com/baprx/zed-yaml-multi-schema/compare/v0.2.0...v0.2.1) (2026-08-26)


### Bug Fixes

* underline full diagnostic line and drop trailing blank line from structure snippet ([d02c1f6](https://github.com/baprx/zed-yaml-multi-schema/commit/d02c1f67bf9810a75883ccca0d777d73a0974ad2))

## [0.2.0](https://github.com/baprx/zed-yaml-multi-schema/compare/v0.1.1...v0.2.0) (2026-08-26)


### Features

* seed object structure via snippets, line-level diagnostics, and fix completion after sibling keys ([10ac8d5](https://github.com/baprx/zed-yaml-multi-schema/commit/10ac8d54ac138431c1d5ed4f9fbe159b13a0fec2))

## [0.1.1](https://github.com/baprx/zed-yaml-multi-schema/compare/v0.1.0...v0.1.1) (2026-08-26)


### Bug Fixes

* revalidate on textDocument/didChange so type errors appear while editing ([bc1caf1](https://github.com/baprx/zed-yaml-multi-schema/commit/bc1caf18342e046c88a33ef94902fb27d0d4dde1))
* emit valid LSP messages for diagnostics and completion kinds ([1172169](https://github.com/baprx/zed-yaml-multi-schema/commit/1172169bb1d187bca396ff4d266c67e2f70ace0f))
* include jsonrpc 2.0 in LSP messages so Zed can deserialize them ([2b5ae7e](https://github.com/baprx/zed-yaml-multi-schema/commit/2b5ae7ee0acccf10ac230816d8519b769d0ba240))
* compile Zed extension for wasm32-wasip2 ([608ebe0](https://github.com/baprx/zed-yaml-multi-schema/commit/608ebe00bf47436607928ce10b96751840c8a29f))
* retry failed schema references on change and add SC-001 coverage ([5631956](https://github.com/baprx/zed-yaml-multi-schema/commit/563195663f5cd109b2a5cb6bb2d1e0a3b46a9927))

## 0.1.0 (2026-08-26)


### Features

* add multi-schema YAML validation for Helm umbrella charts ([f173e10](https://github.com/baprx/zed-yaml-multi-schema/commit/f173e105d3dc1d7add0dd617eec19c2da49d20ed))
