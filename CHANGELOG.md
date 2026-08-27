# Changelog

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
