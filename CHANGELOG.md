# Changelog

## [0.2.0](https://github.com/NavistAu/onvif-server/compare/v0.1.0...v0.2.0) (2026-06-01)


### Features

* **01-foundation-01:** implement OnvifError with SOAP fault mapping and token constants ([d328c1f](https://github.com/NavistAu/onvif-server/commit/d328c1f45c5a09aa38b049edb4c1f066f0f05172))
* **01-foundation-01:** scaffold crate with Cargo.toml, module structure, and stub types ([7c2b794](https://github.com/NavistAu/onvif-server/commit/7c2b794131cdc3c86e946685b68a94bc8d8ff9c1))
* **01-foundation-02:** bundle ONVIF WSDLs and implement EmbeddedWsdlLoader ([45bcf3c](https://github.com/NavistAu/onvif-server/commit/45bcf3c4fcd32e0cb2d8284430b056695c270432))
* **01-foundation-02:** implement five ONVIF service traits with not_implemented() defaults ([536cfb3](https://github.com/NavistAu/onvif-server/commit/536cfb3ffdda6ea53424dc486450d230a8a7701d))
* **01-foundation-02:** type strategy Option B — hand-written stubs, serde dep, WsdlLoader re-export ([e6a5132](https://github.com/NavistAu/onvif-server/commit/e6a513259e9bf8860d25936ad38fdb8fd1d1c073))
* **01-foundation-03:** enable all test stubs — full suite green (6 passed, 0 ignored) ([57a2259](https://github.com/NavistAu/onvif-server/commit/57a2259710711363122a0db6e3f1dd5a4b31bbd8))
* **01-foundation-03:** implement OnvifServerBuilder with service registration and auth config ([4310a90](https://github.com/NavistAu/onvif-server/commit/4310a90e3e8e7b8cb56eac31663711aa539833be))
* **02-01:** expand types, update trait signatures, add test stubs ([0ee4c8e](https://github.com/NavistAu/onvif-server/commit/0ee4c8e621548ea331821c2ee61833f7676450b4))
* **02-01:** implement DeviceServiceHandler with 4 core operations ([21d6d7c](https://github.com/NavistAu/onvif-server/commit/21d6d7c46c78bab560221a61b97e8832b0099286))
* **02-02:** implement GetScopes, GetHostname, GetNetworkInterfaces handlers ([3a773cb](https://github.com/NavistAu/onvif-server/commit/3a773cbb7e621fc93e88a7f3368247a802a0a27c))
* **02-02:** implement OnvifServer::run() with tokio TcpListener and soap-server wiring ([9fb4b4c](https://github.com/NavistAu/onvif-server/commit/9fb4b4cc0b8d0d6e58e2ac2129c884a4007b479e))
* **03-01:** implement MediaServiceHandler with all 6 Media service operations ([9005b85](https://github.com/NavistAu/onvif-server/commit/9005b85f9601668fb907c626649dd8f566ae9f15))
* **03-01:** scaffold constants, types, trait, and test stubs for Media service ([f14b84a](https://github.com/NavistAu/onvif-server/commit/f14b84a052aebc3a18ed2db78f86f051f281b426))
* **03-02:** export media types from crate root; fix test to register media_service ([2f048a3](https://github.com/NavistAu/onvif-server/commit/2f048a36b07eefe6402e12f0704c74deb6184c86))
* **03-02:** wire MediaServiceHandler into run() via Router::merge() ([445fdfd](https://github.com/NavistAu/onvif-server/commit/445fdfd980fd3c9e21b273ad9646ca0e7ac8c417))
* **04-01:** add PTZ types and rewrite PTZService trait with typed signatures ([d186ba8](https://github.com/NavistAu/onvif-server/commit/d186ba8fe646c97281edb42ec76976daa19e8e6d))
* **04-01:** implement PTZServiceHandler with all 15 PTZ operations ([7fca7af](https://github.com/NavistAu/onvif-server/commit/7fca7afcfa27297d7563544e2666efbe8790dbea))
* **04-02:** Frigate autotracker integration test and virtual_ptz example ([c98492f](https://github.com/NavistAu/onvif-server/commit/c98492f4ac3ef7f42a9755e6fe66faa44277abf4))
* **04-02:** wire PTZServiceHandler into OnvifServer::run() at /onvif/ptz_service ([db474bc](https://github.com/NavistAu/onvif-server/commit/db474bc3bc896d084f041629408911c415b3849c))
* **05-01:** EventService typed trait + EventServiceHandler with subscription state ([31fa043](https://github.com/NavistAu/onvif-server/commit/31fa043d325b2fdd41985d26166054c05dd4bfcb))
* **05-01:** ImagingService typed trait + ImagingSettings type + ImagingServiceHandler ([d2920d8](https://github.com/NavistAu/onvif-server/commit/d2920d812ac02bfbaa4983f46fdeb73d101ce240))
* **05-02:** add WS-Discovery UDP task behind discovery feature flag; extend virtual_ptz example with ImagingService and EventService ([ccdf606](https://github.com/NavistAu/onvif-server/commit/ccdf606ab59be24de9f753528cd33e52581f8913))
* **05-02:** wire ImagingServiceHandler and EventServiceHandler into run(); 5-service DeviceServiceHandler ([ea819bc](https://github.com/NavistAu/onvif-server/commit/ea819bc4bef986ab9a155706b6241d1f5bcdfa8a))
* **06-01:** add advertised_host to OnvifServerBuilder and use in XAddr construction ([897ad57](https://github.com/NavistAu/onvif-server/commit/897ad57d9653941d7c134af2629553fc52014e24))
* **06-01:** replace #[ignore] auth stubs with real HTTP-level WS-Security tests ([1b4b0e7](https://github.com/NavistAu/onvif-server/commit/1b4b0e7efc392f4fabaf29a5f1f893a76a40d6c5))


### Bug Fixes

* **01-foundation:** revise plans based on checker feedback ([a992cb1](https://github.com/NavistAu/onvif-server/commit/a992cb1ac9928d6902f94feae8d59fa80903edd2))
* **05-02:** add missing WSDL/XSD stubs for events.wsdl external imports in EmbeddedWsdlLoader ([99f813d](https://github.com/NavistAu/onvif-server/commit/99f813d89dd19f4e7b6e44762d8c12d566af899b))
* address round-1 review blockers ([02703d3](https://github.com/NavistAu/onvif-server/commit/02703d399e503dabea8181cfb07d879508c5a1a3))
* **ptz:** align WSDL and response namespaces to ver20 (zeep compat) ([3cf624b](https://github.com/NavistAu/onvif-server/commit/3cf624bac8f4c12a6f8040ff2b858bbc717e60d2))
* **ptz:** correct GetServices PTZ namespace from ver20 to ver10 ([99104bf](https://github.com/NavistAu/onvif-server/commit/99104bf2f236fd110f2d42ef06979949938b6fae))
* resolve clippy -D warnings issues and fix path-dep CI checkout ([8076cb6](https://github.com/NavistAu/onvif-server/commit/8076cb6ba2da1f49f70cd8eb51e9e1658c2618c3))
* round-2 [#5](https://github.com/NavistAu/onvif-server/issues/5) — events use WS-Addressing subscription addressing + expiry ([389dd6d](https://github.com/NavistAu/onvif-server/commit/389dd6d7abedac5c42ab72a43a233cfd63916ce8))
* round-2 [#8](https://github.com/NavistAu/onvif-server/issues/8) — trait-supplied media profiles; advertise only registered services ([5cffbfa](https://github.com/NavistAu/onvif-server/commit/5cffbfae603e7924e6d2906494baeab11be75cd9))
* round-2 review — escaping, auth optionality, ns-aware parsing, discovery, service optionality ([bebef4f](https://github.com/NavistAu/onvif-server/commit/bebef4f839c1ae5ef34cd029c5853a46d63dc4d1))
* use checkout+symlink for soap-server path dep in CI ([e3f2452](https://github.com/NavistAu/onvif-server/commit/e3f2452ed4de52a6a173893e23c2d3c29b409034))
