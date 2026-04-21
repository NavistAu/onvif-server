# onvif-server Roadmap

## Known Limitations and Deferred Items for v0.2+

These are intentional deviations or gaps identified during the v1.0 internal milestone assessment.

### Type System

- [ ] **No onvif-rs/yaserde dependency** — Spec proposed using lumeohq/onvif-rs XSD-generated types (Option A) or running xsd-parser-rs in build.rs (Option B). Implementation chose Option C: hand-written minimal structs in `src/generated/types.rs` + `format!()` XML construction. This works well for the current surface area but limits reuse if more ONVIF operations are needed.
- [ ] **Trait methods take primitives, not typed request/response structs** — Spec showed traits like `async fn relative_move(&self, req: RelativeMoveRequest)`. Implementation extracts values in the handler layer and passes primitives (f64, String) to trait methods. Simpler but less self-documenting.

### Hardcoded Assumptions

- [ ] **Single profile / single video source** — All media responses are hardcoded to one 1920x1080 H264 profile. Fine for single-camera deployments but limits general-purpose use.
- [ ] **Resolution hardcoded to 1920x1080 @ 30fps** — Not configurable via builder. Should be parameterized if the crate is published for general use.
- [ ] **H264 codec assumption** — VideoEncoderConfiguration hardcodes H264. Some cameras use H265.

### Code Quality

- [ ] **`extract_local_name` duplicated across all 5 service handler files** — Should be extracted to a shared utility (possibly in soap-server).
- [ ] **`build.rs` is a stub** — Declares rerun triggers but does nothing. Can be removed unless future codegen is planned.

### Licensing / Legal

- [ ] **ONVIF WSDL/XSD files are not covered by the crate's MIT OR Apache-2.0 license** — The 7 ONVIF-authored files in `wsdl/` carry ONVIF's own terms: verbatim redistribution allowed (with copyright notice), modification prohibited. Before publishing to crates.io: add a `LICENSE-ONVIF` file with the ONVIF terms, and a README notice clarifying that WSDL/XSD files are under ONVIF's license, not the crate license.

### Missing from Spec

- [ ] **Media2 service** — Spec listed `media2.wsdl` in bundled WSDLs. No Media2 trait or handler exists. Not needed for Frigate (uses Media v1) but some newer clients prefer Media2.
- [ ] **DeviceIO service** — Spec listed `deviceio.wsdl`. No trait or handler. Low priority.
