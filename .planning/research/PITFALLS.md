# Pitfalls Research

**Domain:** ONVIF device server (Rust crate)
**Researched:** 2026-04-05
**Confidence:** HIGH (ONVIF-specific pitfalls verified through spec docs, real client source code, and open issue trackers)

---

## Critical Pitfalls

### Pitfall 1: GetSystemDateAndTime Requires Authentication

**What goes wrong:**
The server returns a 401 or a WS-Security authentication fault for `GetSystemDateAndTime` requests that arrive without credentials. Every ONVIF client that performs digest authentication hits this before it has the server's time. If the server rejects the call, the client cannot compute the WS-Security digest (which mixes `Created` timestamp with the password), so _all subsequent authenticated requests also fail_.

**Why it happens:**
Middleware or framework-level authentication guards are applied uniformly across all routes. The ONVIF-spec exemption for `GetSystemDateAndTime` is a domain rule that generic HTTP/SOAP frameworks don't know about, so developers forget to carve it out.

**How to avoid:**
Register `GetSystemDateAndTime` as auth-exempt in `soap-server` before wiring any service routes. This is already noted in DESIGN.md but must be enforced in the `OnvifServer::builder()` implementation and verified by an integration test that calls the endpoint with no `Security` header and expects a 200.

**Warning signs:**
- All ONVIF clients fail to authenticate despite correct credentials
- Client logs show `FailedAuthentication` or `NotAuthorized` errors on the very first request
- ONVIF Device Manager can't connect at all

**Phase to address:** Device Management service implementation (the phase that wires `soap-server` routes and auth policy)

---

### Pitfall 2: WS-Security Nonce Too Long or Created Timestamp Too Precise

**What goes wrong:**
Strict camera clients (and stricter _servers_ acting as validators) reject WS-Security `UsernameToken` when the `Nonce` is an oversized base64 string (e.g., a base64-encoded UUID string) or when `Created` includes nanosecond precision (`2024-02-04T07:41:57.802907700+00:00`). This is a confirmed issue in `onvif-rs`'s own issue tracker (#114).

**Why it happens:**
UUIDs are convenient unique tokens but produce long base64 output when the UUID _string_ (not bytes) is encoded. Similarly, Rust's `chrono` timestamps include subsecond precision that exceeds what ONVIF clients expect. Strict clients that validate field lengths reject the token.

**How to avoid:**
- Generate nonces as raw random bytes (e.g., 20 bytes) then base64-encode those bytes — producing a compact `~28 character` string, matching ONVIF Device Manager's observed format (`HHsjhr5rbEG8yrFaWySoEzgAAAAAAA==`)
- Format `Created` as `YYYY-MM-DDTHH:MM:SS.sssZ` (millisecond precision, UTC Z suffix) — never include nanoseconds or timezone offsets

**Warning signs:**
- Authentication succeeds with ONVIF Device Manager but fails with HikVision/Dahua hardware
- `NotAuthorized` errors with no obvious cause when credentials are correct

**Phase to address:** WS-Security layer (either in `soap-server` or the ONVIF auth wiring phase)

---

### Pitfall 3: SOAP Fault ter: Namespace Not Declared in Response

**What goes wrong:**
python-zeep (which Frigate uses) throws `XMLParseError: No namespace defined for 'ter'` when the server returns a SOAP fault with a subcode like `<Code Value="ter:InvalidArgVal"/>` but the `ter` namespace prefix is not declared on the envelope element. The fault is technically malformed XML.

**Why it happens:**
The `ter` namespace (`http://www.onvif.org/ver10/error`) must be explicitly declared on the `Envelope` element (or at least on the `Fault` element). Developers hand-building fault XML forget that namespace prefixes are not globally known — they must be declared in scope. Generic SOAP fault templates often omit ONVIF-specific namespaces.

**How to avoid:**
Ensure every SOAP fault envelope declares `xmlns:ter="http://www.onvif.org/ver10/error"`. Write a test that parses every fault variant with a strict XML namespace validator. The `not_implemented()` default handler for trait methods must also produce spec-compliant faults.

**Warning signs:**
- python-zeep or python-onvif-zeep throws `XMLParseError` or `lxml` namespace errors on any error response
- Works with one client but breaks another (different XML parsers have different namespace strictness)

**Phase to address:** Core SOAP fault handling (early, before any service is wired)

---

### Pitfall 4: Frigate Silently Drops PTZ if GetProfiles Returns No Profile with PTZConfiguration

**What goes wrong:**
Frigate's `onvif.py` iterates profiles and selects the first one that has _both_ `VideoEncoderConfiguration` _and_ `PTZConfiguration` present, with `PTZConfiguration` containing a non-null `DefaultContinuousPanTiltVelocitySpace` or `DefaultContinuousZoomVelocitySpace`. If no profile satisfies all conditions, Frigate logs "no appropriate ONVIF profiles found" and disables PTZ entirely — with no further indication of why.

**Why it happens:**
A minimal `GetProfiles` implementation that returns a profile with a `PTZConfiguration` struct but leaves `DefaultContinuousPanTiltVelocitySpace` empty (because the implementor thought it was optional) fails Frigate's validation check silently.

**How to avoid:**
Return exactly one profile (or at least one) with:
- `VideoEncoderConfiguration` populated
- `PTZConfiguration` populated with `DefaultContinuousPanTiltVelocitySpace` set to the TranslationSpaceFov URI (`http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationSpaceFov`)
- `token` attribute matching what PTZ operations will receive

Verify with a test that reads Frigate's profile-selection logic directly.

**Warning signs:**
- Frigate logs "No appropriate Onvif profiles found for camera"
- PTZ controls are absent from the Frigate UI
- Manual ONVIF Device Manager query shows profiles, but Frigate still ignores them

**Phase to address:** Media service and PTZ service implementation phase (Frigate compatibility milestone)

---

### Pitfall 5: TranslationSpaceFov URI Not Present in GetConfigurationOptions

**What goes wrong:**
Frigate checks `GetConfigurationOptions` response for `Spaces.RelativePanTiltTranslationSpace` entries containing a URI matching `TranslationSpaceFov`. If the URI is absent or misspelled, Frigate refuses to use `RelativeMove` for autotracking and falls back to failing — even if `RelativeMove` itself would work.

**Why it happens:**
The exact URI `http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationSpaceFov` must appear as the `URI` field in a `Space2DDescription` inside `RelativePanTiltTranslationSpace`. Implementors who approximate the response structure or copy-paste a slightly different URI break this check. The URI is case-sensitive.

**How to avoid:**
Treat the TranslationSpaceFov URI as a constant string — define it once in the codebase:
```rust
pub const TRANSLATION_SPACE_FOV: &str =
    "http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationSpaceFov";
```
Use this constant in both `GetNodes` and `GetConfigurationOptions` responses. Write an integration test that calls `GetConfigurationOptions` and asserts this exact URI appears in the response.

**Warning signs:**
- Frigate's autotracker does not activate even though PTZ controls appear in the UI
- Frigate logs show that `RelativeMove` capability was not detected
- ONVIF Device Manager shows the node, but Frigate behaves as if the camera is non-PTZ

**Phase to address:** PTZ service implementation (before Frigate integration testing)

---

### Pitfall 6: GetServiceCapabilities MoveStatus Not Advertised

**What goes wrong:**
Frigate calls `GetServiceCapabilities` and uses `find_by_key(vars(service_capabilities), "MoveStatus")` to locate the `MoveStatus` attribute. If the response omits `<Capabilities MoveStatus="true"/>` (or returns a struct where `MoveStatus` serializes to absent/null), Frigate skips polling `GetStatus` entirely — meaning it never knows when a move completes, causing tracking to overshoot.

**Why it happens:**
`GetServiceCapabilities` is a frequently-unimplemented operation. Default `not_implemented()` returns a SOAP fault, which Frigate may catch and treat as "feature not supported." Even a partial implementation that returns a `Capabilities` element without `MoveStatus="true"` fails the check.

**How to avoid:**
Implement `GetServiceCapabilities` for `PTZService` to explicitly return `MoveStatus="true"`. Do not rely on a default or stub response. This is a required operation for Frigate compatibility.

**Warning signs:**
- Autotracker overshoots objects with no sign of waiting for IDLE status
- Frigate logs show it is not polling `GetStatus`
- The `GetServiceCapabilities` call returns a fault or an empty capabilities struct

**Phase to address:** PTZ service implementation (Frigate compatibility milestone)

---

### Pitfall 7: XML Namespace Prefix Collisions in yaserde-Serialized Responses

**What goes wrong:**
yaserde (used by onvif-rs types) has documented issues where namespace declarations are not placed correctly on output elements, or where elements from different namespaces are incorrectly deserialized as if they belong to the wrong namespace. The result is malformed XML that some strict parsers reject, or responses where required fields deserialize to empty values on the client side.

**Why it happens:**
yaserde's namespace handling was designed for a client/deserializing use case. Server-side serialization exposes edge cases: nested elements may re-declare namespaces in unexpected ways, or the top-level envelope namespace declarations may be incomplete. The onvif-rs crate itself works around this by using `xmltree` for intermediate parsing — indicating the library's own authors encountered these issues.

**How to avoid:**
- Test every response type against python-zeep's strict XML parser (it validates namespace correctness)
- Log the raw XML of responses during development and inspect it manually
- If a specific type serializes incorrectly, consider a manual `Serialize` implementation for that type rather than relying on the derive macro

**Warning signs:**
- python-zeep raises namespace or parse errors on specific operations only
- Values in responses appear `None` on the client despite being set on the server
- Inconsistent behavior between clients (Python strict parser vs. lenient XML readers)

**Phase to address:** Type definition research phase; verify during each service's implementation

---

### Pitfall 8: WS-Security Timestamp Freshness Window Rejection

**What goes wrong:**
ONVIF servers that enforce the 5-minute nonce freshness window (as the spec recommends) will reject replayed or stale requests. Conversely, a server that does _not_ enforce this creates a replay vulnerability. The real operational pitfall: clients that call `GetSystemDateAndTime` to sync time, then compute the digest, but have network latency or slow startup paths can be pushed outside the acceptable window by the time the authenticated request arrives.

**Why it happens:**
The spec recommends a 5-minute freshness window. Strict implementations that use a 60-second window (as some devices do) cause clients that didn't read the exact spec to fail. Additionally, any clock drift on the server causes all client requests to fail even when the client has synced via `GetSystemDateAndTime`.

**How to avoid:**
- Use a configurable freshness window, defaulting to 5 minutes per spec
- Keep the server clock synchronized (document that the host needs NTP)
- Accept both `wsse:Password Type="PasswordDigest"` and `PasswordText` per spec
- In tests, verify that a request with `Created` exactly 299 seconds old is accepted and 301 seconds old is rejected

**Warning signs:**
- Authentication failures that appear and disappear intermittently
- Works locally but fails in production (clock drift)
- Failures correlate with specific times of day

**Phase to address:** WS-Security implementation in `soap-server` (verified here in onvif-server integration tests)

---

### Pitfall 9: Profile Token and PTZConfiguration Token Inconsistency

**What goes wrong:**
PTZ operations (`RelativeMove`, `GetStatus`, `GotoPreset`, etc.) accept a `ProfileToken`. The server looks up the `PTZConfiguration` for that profile and uses its token internally. If the profile's `PTZConfiguration.token` does not match any token in `GetConfigurations`, or if different operations use different token namespaces/formats, clients receive `ter:NoProfile` or `ter:NoPTZProfile` faults for operations that should work.

**Why it happens:**
Tokens are arbitrary strings but must be consistent across all responses. A common mistake is returning a hardcoded token string in `GetProfiles` but a different hardcoded string in `GetConfigurations`, causing cross-operation lookups to fail. Also, some clients pass the profile token directly to `GetNode` (which expects a _node_ token, not a profile token) — returning a sensible error requires understanding the distinction.

**How to avoid:**
Define all tokens as constants:
```rust
pub const PROFILE_TOKEN: &str = "main";
pub const PTZ_CONFIGURATION_TOKEN: &str = "ptz_config_1";
pub const PTZ_NODE_TOKEN: &str = "ptz_node_1";
```
Verify in integration tests that a `RelativeMove` request with the profile token from `GetProfiles` succeeds end-to-end.

**Warning signs:**
- Specific PTZ operations return `NoProfile` despite profiles being present
- Operations work in sequence when warm but fail on cold first calls
- `GetStatus` fails with token errors after a successful `RelativeMove`

**Phase to address:** PTZ service implementation

---

### Pitfall 10: GetCapabilities and GetServices Must Both Be Implemented

**What goes wrong:**
Older ONVIF clients use `GetCapabilities` (the pre-2.0 API) to discover service endpoints. Newer clients use `GetServices`. A server that only implements `GetServices` will fail to connect with ONVIF Device Manager and some NVR software. A server that only implements `GetCapabilities` looks like an old device to modern clients.

**Why it happens:**
The spec deprecated `GetCapabilities` in favor of `GetServices`, leading implementors to skip the older endpoint. But the installed base of ONVIF clients that use `GetCapabilities` is enormous.

**How to avoid:**
Implement both. The `GetCapabilities` response must include accurate `XAddr` URLs for each service (PTZ, Media, Imaging, Events) that match the URLs the server actually listens on. The URLs must be absolute (including `http://host:port/onvif/...`), not relative paths.

**Warning signs:**
- ONVIF Device Manager shows "connection failed" or empty capability list
- NVR auto-discovery finds the device but no services appear
- python-onvif-zeep works but a Java/C# client cannot enumerate services

**Phase to address:** Device Management service implementation

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Hardcoded profile/node/config tokens as inline string literals | Fast to prototype | Tokens used in multiple response types diverge silently | Never — define as constants from day 1 |
| Returning empty `Capabilities` struct from `GetServiceCapabilities` | Avoids implementing the operation | Frigate silently disables autotracking; other clients may also silently downgrade | Never for PTZ service |
| Skipping auth exemption for `GetSystemDateAndTime` | Simpler uniform auth middleware | All clients fail to authenticate | Never |
| Using UUID v4 string (not bytes) as nonce | Convenient unique token | Long nonce causes strict cameras to reject auth | Never — use raw random bytes |
| Nanosecond precision in `Created` timestamp | Rust's default `chrono` formatting | Certain HikVision/Dahua cameras reject auth | Never — truncate to milliseconds |
| Implementing only `GetServices`, not `GetCapabilities` | Half the work | ONVIF Device Manager and older NVRs cannot connect | Never for a general-purpose library |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| python-onvif-zeep (Frigate) | Returning profiles where `PTZConfiguration.DefaultContinuousPanTiltVelocitySpace` is null | Populate `DefaultContinuousPanTiltVelocitySpace` with the TranslationSpaceFov URI |
| python-onvif-zeep (Frigate) | Omitting `MoveStatus="true"` attribute on `GetServiceCapabilities` `Capabilities` element | Explicitly return `Capabilities { MoveStatus: true, ... }` |
| python-zeep XML parser | SOAP fault with `ter:` prefix but no `xmlns:ter` declaration on envelope | Declare `xmlns:ter="http://www.onvif.org/ver10/error"` on every fault envelope |
| Any ONVIF client | Service URLs in `GetCapabilities`/`GetServices` are relative paths | Always return absolute URLs: `http://{host}:{port}/onvif/{service}` |
| ONVIF Device Manager | `GetSystemDateAndTime` requires authentication | Exempt this operation from WS-Security validation |
| Strict hardware cameras (HikVision, Dahua) | Long nonce or nanosecond `Created` in WS-Security header | Use 20-byte random nonce, millisecond UTC `Created` |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Blocking operation in async trait handler | Tokio executor threads starved; `GetStatus` polls time out | Ensure all I/O in trait implementations uses async; use `tokio::task::spawn_blocking` for CPU-bound work | Immediately under any concurrent load |
| Nonce cache growing unbounded | Server memory grows proportional to uptime; DoS vector | Use a bounded LRU cache with TTL eviction matching the freshness window | After hours of sustained authenticated traffic |
| Repeated WSDL file reads on every `GetWsdl` request | High latency for WSDL serving; unnecessary disk I/O | Load bundled WSDLs into memory at startup; serve from `&'static str` or `Arc<str>` | After moderate client use (NVR startup auto-fetches all WSDLs) |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Not enforcing nonce replay prevention | Replay attack: attacker records an authenticated ONVIF request and reuses it (CVE-2022-30563 pattern in Dahua cameras) | Cache used nonces for the freshness window duration; reject duplicates |
| Not enforcing `Created` timestamp freshness | Same replay attack with stale credentials | Reject `Created` timestamps older than 5 minutes (or configurable window) |
| Logging raw `Security` header including `Password` or `PasswordDigest` | Credential exposure in server logs | Strip or redact WS-Security header content in any debug logging |
| Accepting `PasswordText` type without rate limiting | Brute-force password attack over ONVIF | Document that production deployments should sit behind a firewall; optionally add lockout logic |

---

## "Looks Done But Isn't" Checklist

- [ ] **GetSystemDateAndTime exempt from auth:** Verify with a test that calls the endpoint with no `Authorization`/`Security` header and receives a 200 response.
- [ ] **Frigate profile validation:** Check that at least one profile returned by `GetProfiles` has both `VideoEncoderConfiguration` and `PTZConfiguration` with `DefaultContinuousPanTiltVelocitySpace` set to the TranslationSpaceFov URI.
- [ ] **TranslationSpaceFov URI exact match:** The URI in `GetConfigurationOptions` response must exactly equal `http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationSpaceFov` — check for trailing slashes or version variations.
- [ ] **SOAP faults declare ter: namespace:** Confirm `xmlns:ter="http://www.onvif.org/ver10/error"` appears on the envelope of every fault response; parse it with a strict XML validator.
- [ ] **GetServiceCapabilities returns MoveStatus true:** Parse the actual XML response, not just assert the Rust struct; verify the attribute appears in the serialized output.
- [ ] **WS-Security nonce format:** Log the actual nonce and Created values generated; confirm nonce is ~28 base64 chars (20 raw bytes), Created is millisecond precision ending in `Z`.
- [ ] **Token consistency:** Write a test that calls GetProfiles, extracts the PTZ profile token, passes it to RelativeMove, and asserts success — proving the token flows correctly across services.
- [ ] **Both GetCapabilities and GetServices implemented:** Test with ONVIF Device Manager (uses GetCapabilities) AND python-onvif-zeep (uses GetServices).

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Auth exemption missing for GetSystemDateAndTime | LOW | Add route exemption, no API changes required |
| Nonce/Created format wrong | LOW | Change nonce generation and timestamp formatting in WS-Security layer |
| ter: namespace missing from faults | LOW | Fix fault envelope template; affects all faults uniformly |
| Profile structure wrong for Frigate | MEDIUM | Requires understanding Frigate's exact validation path; change GetProfiles and GetConfigurationOptions response structure |
| Token inconsistency across services | MEDIUM | Refactor to use shared token constants; trace through all response types that embed tokens |
| yaserde namespace serialization bug | HIGH | May require manual Serialize implementations for affected types or switching to a different XML serialization approach |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| GetSystemDateAndTime auth exempt | Device Management service wiring | Test: unauthenticated request returns 200 |
| Nonce/Created format | WS-Security implementation (soap-server layer) | Log and inspect actual nonce/Created values; test against strict camera |
| SOAP fault ter: namespace | Core SOAP fault infrastructure | Parse every fault type with python-zeep strict mode |
| Frigate profile validation | Media + PTZ service implementation | Run Frigate's profile-selection logic against test response |
| TranslationSpaceFov URI | PTZ service implementation | Assert exact URI string in GetConfigurationOptions response |
| GetServiceCapabilities MoveStatus | PTZ service implementation | Assert MoveStatus="true" in serialized XML |
| yaserde namespace pitfalls | Type definition selection (onvif-rs vs. generated) | End-to-end test every response type with python-zeep |
| Timestamp freshness window | WS-Security implementation | Test boundary: 299s accepted, 301s rejected |
| Token inconsistency | PTZ service implementation | End-to-end flow test: GetProfiles token through RelativeMove |
| GetCapabilities + GetServices both required | Device Management service | Test with ONVIF Device Manager AND python-onvif-zeep |

---

## Sources

- [Frigate ptz/onvif.py source code](https://github.com/blakeblackshear/frigate/blob/dev/frigate/ptz/onvif.py) — ground truth for Frigate's ONVIF call sequence and validation logic
- [Frigate autotracking documentation](https://docs.frigate.video/configuration/autotracking/) — TranslationSpaceFov requirement documentation
- [onvif-rs issue #114: Nonce and Created too long](https://github.com/lumeohq/onvif-rs/issues/114) — concrete nonce format issue with HikVision cameras
- [python-zeep issue #1205: XMLParseError No namespace defined for 'ter'](https://github.com/mvantellingen/python-zeep/issues/1205) — ter: namespace fault issue in the wild
- [CVE-2022-30563: Dahua ONVIF replay vulnerability](https://www.nozominetworks.com/blog/vulnerability-in-dahua-s-onvif-implementation-threatens-ip-camera-security) — nonce replay attack via WS-UsernameToken
- [OASIS WS-UsernameToken Profile 1.1.1](https://docs.oasis-open.org/wss-m/wss/v1.1.1/os/wss-UsernameTokenProfile-v1.1.1-os.html) — authoritative nonce and freshness window specification
- [ONVIF Core Specification 25.12](https://www.onvif.org/specs/core/ONVIF-Core-Specification.pdf) — SOAP fault format, ter: namespace, auth exemption rules
- [IPVM discussion: GetSystemDateAndTime auth exempt](https://ipvm.com/discussions/onvif-gurus-is-there-a-valid-reason-for-getsystemdate-to-require-authentication) — community confirmation that many devices incorrectly require auth here
- [Hawkeye217 FOV detection gist](https://gist.github.com/hawkeye217/152a1d4ba80760dac95d46e143d37112) — how to check TranslationSpaceFov support (Frigate maintainer's own script)
- [yaserde namespace issues](https://github.com/luminvent/yaserde/issues/157) — documented deserialization namespace bugs

---

*Pitfalls research for: Rust ONVIF device server crate*
*Researched: 2026-04-05*
