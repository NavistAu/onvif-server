#!/usr/bin/env python3
"""
Extract embedded <xs:schema> from ONVIF WSDL files and produce standalone *-body.xsd files.
Also rewrites schemaLocation attributes in shared XSDs to local filenames.

CRITICAL: All in-scope ancestor namespace declarations from wsdl:definitions,
wsdl:types, and xs:schema itself are preserved on the extracted <xs:schema>
element so that prefix references (e.g. tt:, tds:, tptz:) remain valid.

Usage:
  python3 extract-schemas.py <wsdl_dir> <output_dir>

Writes to <output_dir>/:
  device-body.xsd, media-body.xsd, imaging-body.xsd, ptz-body.xsd, events-body.xsd
  onvif.xsd, common.xsd, ws-addr.xsd, soap-envelope.xsd, xmlmime.xsd,
  xop-include.xsd, wsn-b2.xsd, wsn-t1.xsd  (with local schemaLocation rewrites)
"""

import os
import sys
import re

WSDL_NS = "http://schemas.xmlsoap.org/wsdl/"
XS_NS   = "http://www.w3.org/2001/XMLSchema"

# Map schemaLocation values -> local filename (exact or suffix matches)
SCHEMA_LOCATION_REWRITES = {
    # onvif.xsd - various relative paths used across WSDLs
    "../../../ver10/schema/onvif.xsd": "onvif.xsd",
    "onvif.xsd": "onvif.xsd",
    # ws-addr
    "http://www.w3.org/2005/08/addressing/ws-addr.xsd": "ws-addr.xsd",
    # wsn-t1
    "http://docs.oasis-open.org/wsn/t-1.xsd": "wsn-t1.xsd",
    # wsn-b2
    "http://docs.oasis-open.org/wsn/b-2.xsd": "wsn-b2.xsd",
    # soap-envelope (onvif.xsd imports this)
    "https://www.w3.org/2003/05/soap-envelope": "soap-envelope.xsd",
    "http://www.w3.org/2003/05/soap-envelope": "soap-envelope.xsd",
    # xmlmime
    "https://www.w3.org/2005/05/xmlmime": "xmlmime.xsd",
    "http://www.w3.org/2005/05/xmlmime": "xmlmime.xsd",
    # xop-include
    "https://www.w3.org/2004/08/xop/include": "xop-include.xsd",
    "http://www.w3.org/2004/08/xop/include": "xop-include.xsd",
    # common.xsd (already local in onvif.xsd, keep it)
    "common.xsd": "common.xsd",
}

# Shared XSDs to copy (with schemaLocation rewriting)
SHARED_XSDS = [
    "onvif.xsd",
    "common.xsd",
    "ws-addr.xsd",
    "soap-envelope.xsd",
    "xmlmime.xsd",
    "xop-include.xsd",
    "wsn-b2.xsd",
    "wsn-t1.xsd",
]


def get_opening_tag_text(raw_xml, tag_name):
    """Return the full opening tag text for the FIRST element with the given tag name."""
    start = raw_xml.find('<' + tag_name)
    if start < 0:
        return ''
    i = start + 1
    in_quote = False
    quote_char = None
    while i < len(raw_xml):
        c = raw_xml[i]
        if in_quote:
            if c == quote_char:
                in_quote = False
        else:
            if c in ('"', "'"):
                in_quote = True
                quote_char = c
            elif c == '>':
                return raw_xml[start:i+1]
        i += 1
    return ''


def extract_xmlns_from_tag(tag_text):
    """Extract xmlns:prefix="uri" declarations. Returns dict prefix->uri ('' = default)."""
    result = {}
    for m in re.finditer(r'xmlns(?::([a-zA-Z0-9_\-\.]+))?=["\']([^"\']*)["\']', tag_text):
        prefix = m.group(1) or ''
        uri = m.group(2)
        result[prefix] = uri
    return result


def collect_all_ancestor_ns(raw_xml):
    """
    Collect namespace declarations from wsdl:definitions, wsdl:types, and xs:schema.
    Inner scope (xs:schema) wins on conflicts.
    """
    ns_map = {}
    for tag in ('wsdl:definitions', 'wsdl:types', 'xs:schema'):
        tag_text = get_opening_tag_text(raw_xml, tag)
        if tag_text:
            ns_map.update(extract_xmlns_from_tag(tag_text))
    return ns_map


def extract_schema_block(raw_xml):
    """
    Extract the raw text of the first <xs:schema ...>...</xs:schema> block
    inside <wsdl:types>. Returns raw text or None.
    """
    types_start = raw_xml.find('<wsdl:types')
    if types_start < 0:
        return None
    schema_start = raw_xml.find('<xs:schema', types_start)
    if schema_start < 0:
        return None

    depth = 0
    i = schema_start
    while i < len(raw_xml):
        if raw_xml[i] == '<':
            if raw_xml[i:i+11] == '</xs:schema':
                depth -= 1
                if depth == 0:
                    end = raw_xml.find('>', i) + 1
                    return raw_xml[schema_start:end]
            elif raw_xml[i:i+10] == '<xs:schema':
                next_char = raw_xml[i+10] if i+10 < len(raw_xml) else ''
                if next_char in (' ', '\t', '\n', '\r', '/', '>'):
                    depth += 1
        i += 1
    return None


def rewrite_schema_locations(text):
    """
    Rewrite schemaLocation values in xs:import and xs:include elements to local names.
    Only touches elements matching xs:import or xs:include.
    """
    def replace_sl(m):
        attr = m.group(1)
        quote = m.group(2)
        value = m.group(3)
        # Exact match
        new_value = SCHEMA_LOCATION_REWRITES.get(value)
        if new_value is None:
            # Suffix/substring match
            for pattern, replacement in SCHEMA_LOCATION_REWRITES.items():
                if value == pattern or value.endswith('/' + pattern) or value.endswith(pattern):
                    new_value = replacement
                    break
        if new_value is None:
            new_value = value
        return f'{attr}={quote}{new_value}{quote}'

    result = []
    i = 0
    while i < len(text):
        imp = text.find('<xs:import', i)
        inc = text.find('<xs:include', i)
        candidates = [x for x in [imp, inc] if x >= 0]
        if not candidates:
            result.append(text[i:])
            break
        next_tag = min(candidates)
        result.append(text[i:next_tag])
        end = text.find('>', next_tag) + 1
        element_text = text[next_tag:end]
        element_text = re.sub(
            r'(schemaLocation)=(["\'])([^"\']*)\2',
            replace_sl,
            element_text
        )
        result.append(element_text)
        i = end
    return ''.join(result)


def inject_xmlns_into_schema_tag(schema_text, ns_map):
    """Inject xmlns declarations from ns_map into the opening <xs:schema> tag."""
    m = re.match(r'(<xs:schema\b[^>]*?)(/>|>)', schema_text, re.DOTALL)
    if not m:
        return schema_text
    opening = m.group(1)
    closing = m.group(2)
    rest = schema_text[m.end():]

    parts = []
    for prefix, uri in sorted(ns_map.items()):
        if not uri:
            continue
        attr = f'xmlns:{prefix}' if prefix else 'xmlns'
        # Don't duplicate existing declarations
        if f'{attr}=' not in opening:
            parts.append(f'{attr}="{uri}"')

    extra = (' ' + ' '.join(parts)) if parts else ''
    return opening + extra + closing + rest


def extract_schemas_from_wsdl(wsdl_path, output_dir, service_name):
    """Extract xs:schema from wsdl:types, inject namespaces, rewrite schemaLocations."""
    with open(wsdl_path, 'r', encoding='utf-8') as f:
        raw = f.read()

    ns_map = collect_all_ancestor_ns(raw)
    schema_block = extract_schema_block(raw)
    if schema_block is None:
        print(f"ERROR: could not find xs:schema in {wsdl_path}", file=sys.stderr)
        sys.exit(1)

    schema_block = rewrite_schema_locations(schema_block)
    schema_block = inject_xmlns_into_schema_tag(schema_block, ns_map)

    # Fix UPA (Unique Particle Attribution) violations in extracted body schemas.
    # xs:any namespace="##any" on a sibling particle with explicitly-declared optional
    # elements in the same targetNamespace is ambiguous: both the named element and the
    # wildcard match the element, violating XSD UPA rules.  Xerces (strict) rejects this;
    # xmllint tolerated it.  Replace ##any with ##other so the wildcard only matches
    # elements NOT in the body schema's own namespace (all of which are declared explicitly).
    # This mirrors the ##targetNamespace→##other fix already applied to shared XSDs.
    schema_block = schema_block.replace(
        'namespace="##any"',
        'namespace="##other"'
    )

    output = '<?xml version="1.0" encoding="UTF-8"?>\n' + schema_block + '\n'

    out_path = os.path.join(output_dir, f'{service_name}-body.xsd')
    with open(out_path, 'w', encoding='utf-8') as f:
        f.write(output)
    print(f"Extracted: {out_path}")


# Enhanced wsn-b2.xsd stub content for the oracle.
# Extends the minimal wsdl/ stub with the additional types/elements referenced by
# events.wsdl: AbsoluteOrRelativeTimeType, CurrentTime, TerminationTime,
# NotificationMessage, FixedTopicSet, TopicExpressionDialect.
WSN_B2_ENHANCED = '''\
<?xml version="1.0" encoding="UTF-8"?>
<!-- Enhanced stub for http://docs.oasis-open.org/wsn/b-2 -->
<!-- Oracle copy: extends the minimal wsdl/ stub with elements/types referenced by events.wsdl -->
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2"
           targetNamespace="http://docs.oasis-open.org/wsn/b-2"
           elementFormDefault="qualified">

  <!-- Types used by events.wsdl -->
  <xs:complexType name="FilterType">
    <xs:sequence>
      <xs:any namespace="##other" minOccurs="0" maxOccurs="unbounded" processContents="lax"/>
    </xs:sequence>
  </xs:complexType>

  <xs:complexType name="NotificationMessageHolderType">
    <xs:sequence>
      <xs:any namespace="##any" minOccurs="0" maxOccurs="unbounded" processContents="lax"/>
    </xs:sequence>
  </xs:complexType>

  <!-- AbsoluteOrRelativeTimeType: used as type of InitialTerminationTime -->
  <xs:simpleType name="AbsoluteOrRelativeTimeType">
    <xs:union memberTypes="xs:dateTime xs:duration"/>
  </xs:simpleType>

  <!-- Element declarations referenced by events.wsdl -->
  <xs:element name="CurrentTime" type="xs:dateTime"/>
  <xs:element name="TerminationTime" type="xs:dateTime" nillable="true"/>
  <xs:element name="NotificationMessage" type="wsnt:NotificationMessageHolderType"/>
  <xs:element name="FixedTopicSet" type="xs:boolean"/>
  <xs:element name="TopicExpressionDialect" type="xs:anyURI"/>

</xs:schema>
'''


def collect_global_names(xsd_text):
    """
    Return the set of (symbol_space, name) pairs for every top-level global component
    that is a DIRECT child of the <xs:schema> root element.

    Symbol spaces follow XSD spec:
      'type'           -- xs:complexType and xs:simpleType (share one symbol space)
      'element'        -- xs:element
      'attribute'      -- xs:attribute
      'group'          -- xs:group
      'attributeGroup' -- xs:attributeGroup

    Only direct children (depth == 1 inside xs:schema) are counted; nested type
    definitions do NOT create global components.
    """
    # XSD tags that define named global components and their symbol space
    GLOBAL_TAGS = {
        'xs:complexType':   'type',
        'xs:simpleType':    'type',
        'xs:element':       'element',
        'xs:attribute':     'attribute',
        'xs:group':         'group',
        'xs:attributeGroup':'attributeGroup',
    }

    names = set()

    # Find the schema opening tag
    schema_start = xsd_text.find('<xs:schema')
    if schema_start < 0:
        return names

    # Skip to end of the opening <xs:schema ...> tag (may be multi-line)
    i = schema_start + len('<xs:schema')
    in_quote = False
    quote_char = None
    while i < len(xsd_text):
        c = xsd_text[i]
        if in_quote:
            if c == quote_char:
                in_quote = False
        else:
            if c in ('"', "'"):
                in_quote = True
                quote_char = c
            elif c == '>':
                i += 1
                break
        i += 1

    depth = 1  # we are now inside <xs:schema>

    while i < len(xsd_text):
        if xsd_text[i] != '<':
            i += 1
            continue

        # Skip XML comments
        if xsd_text[i:i+4] == '<!--':
            end = xsd_text.find('-->', i)
            i = end + 3 if end >= 0 else len(xsd_text)
            continue

        # Closing tag
        if xsd_text[i+1:i+2] == '/':
            tag_end = xsd_text.find('>', i)
            tag_end = tag_end + 1 if tag_end >= 0 else len(xsd_text)
            # Only track xs:* closing tags for depth — non-xs closing tags (e.g. HTML
            # <li>, <ul> inside xs:documentation) have no matching xs: open to pair with.
            close_m = re.match(r'</(xs:[a-zA-Z]+)', xsd_text[i:])
            if close_m:
                depth -= 1
            i = tag_end
            continue

        # Opening or self-closing tag: read to '>'
        tag_end = i + 1
        in_q = False
        q_char = None
        while tag_end < len(xsd_text):
            c = xsd_text[tag_end]
            if in_q:
                if c == q_char:
                    in_q = False
            else:
                if c in ('"', "'"):
                    in_q = True
                    q_char = c
                elif c == '>':
                    break
            tag_end += 1

        tag_text = xsd_text[i:tag_end + 1]
        is_self_closing = tag_text.rstrip().endswith('/>')

        # Extract xs:* tag name (non-xs opening tags like <ul>, <li> in docs are ignored)
        m = re.match(r'<(xs:[a-zA-Z]+)', tag_text)
        tag_name = m.group(1) if m else None

        # Record global component if this is a direct child of xs:schema
        if depth == 1 and tag_name in GLOBAL_TAGS:
            nm = re.search(r'\bname=["\']([^"\']+)["\']', tag_text)
            if nm:
                names.add((GLOBAL_TAGS[tag_name], nm.group(1)))

        if not is_self_closing and tag_name is not None:
            depth += 1

        i = tag_end + 1

    return names


def _find_tag_end(text, start):
    """Find the position just past the '>' that closes the tag starting at text[start].
    Returns (tag_text, end_pos, is_self_closing).
    """
    i = start + 1
    in_q = False
    q_char = None
    while i < len(text):
        c = text[i]
        if in_q:
            if c == q_char:
                in_q = False
        else:
            if c in ('"', "'"):
                in_q = True
                q_char = c
            elif c == '>':
                end_pos = i + 1
                tag_text = text[start:end_pos]
                is_self_closing = tag_text.rstrip().endswith('/>')
                return tag_text, end_pos, is_self_closing
        i += 1
    # Unterminated tag — consume rest
    return text[start:], len(text), True


def skip_element_subtree(text, start):
    """
    Given that text[start] is the start of an already-seen opening tag (non-self-closing),
    skip everything up to and including the matching closing tag.
    Returns the position just past the closing tag.

    Uses depth tracking to handle same-tag nesting correctly.
    """
    # Get the tag name to track matching close
    m = re.match(r'<([^\s>/]+)', text[start:])
    if not m:
        return start + 1
    outer_tag = m.group(1)

    depth = 1
    i = start
    # Find end of the opening tag first
    _, i, _ = _find_tag_end(text, start)

    while i < len(text) and depth > 0:
        if text[i] != '<':
            i += 1
            continue
        # Comment
        if text[i:i+4] == '<!--':
            end = text.find('-->', i)
            i = end + 3 if end >= 0 else len(text)
            continue
        # Closing tag
        if text[i+1:i+2] == '/':
            close_end = text.find('>', i)
            close_end = close_end + 1 if close_end >= 0 else len(text)
            # Check if this closes our tracked tag
            tag_name_m = re.match(r'</([\w:]+)', text[i:])
            if tag_name_m and tag_name_m.group(1) == outer_tag:
                depth -= 1
            i = close_end
            continue
        # Opening tag
        tag_text, te, is_self_closing = _find_tag_end(text, i)
        if not is_self_closing:
            tag_name_m = re.match(r'<([\w:]+)', tag_text)
            if tag_name_m and tag_name_m.group(1) == outer_tag:
                depth += 1
        i = te

    return i


def strip_colliding_globals(common_text, onvif_names):
    """
    Remove from common_text every top-level <xs:schema> child whose (symbol_space, name)
    appears in onvif_names.  Uses depth-tracking (mirroring extract_schema_block) so that
    nested elements with the same tag name are never mis-counted.

    Returns the deduplicated schema text.
    Preserves all annotations, comments, whitespace, namespace declarations, and unique
    type definitions.
    """
    GLOBAL_TAGS = {
        'xs:complexType':   'type',
        'xs:simpleType':    'type',
        'xs:element':       'element',
        'xs:attribute':     'attribute',
        'xs:group':         'group',
        'xs:attributeGroup':'attributeGroup',
    }

    # Find the schema opening tag boundary
    schema_start = common_text.find('<xs:schema')
    if schema_start < 0:
        return common_text

    # Find end of <xs:schema ...> opening tag (not self-closing)
    _, schema_tag_end, _ = _find_tag_end(common_text, schema_start)

    # Everything up to (and including) the <xs:schema ...> tag is kept verbatim.
    prefix = common_text[:schema_tag_end]

    # Find the </xs:schema> closing tag
    schema_close_start = common_text.rfind('</xs:schema')
    if schema_close_start < 0:
        return common_text  # malformed, return as-is
    schema_close_end = common_text.find('>', schema_close_start) + 1
    suffix = common_text[schema_close_start:schema_close_end]
    # Anything after </xs:schema> (e.g. trailing newline)
    tail = common_text[schema_close_end:]

    # Process the body (between schema open tag and schema close tag)
    body = common_text[schema_tag_end:schema_close_start]

    result = []
    i = 0
    while i < len(body):
        if body[i] != '<':
            result.append(body[i])
            i += 1
            continue

        # XML comment: always keep
        if body[i:i+4] == '<!--':
            end = body.find('-->', i)
            end_pos = end + 3 if end >= 0 else len(body)
            result.append(body[i:end_pos])
            i = end_pos
            continue

        # These should all be top-level children (the body is flat between schema tags)
        tag_text, tag_end, is_self_closing = _find_tag_end(body, i)
        m = re.match(r'<([\w:]+)', tag_text)
        tag_name = m.group(1) if m else None

        if tag_name in GLOBAL_TAGS:
            nm = re.search(r'\bname=["\']([^"\']+)["\']', tag_text)
            if nm:
                key = (GLOBAL_TAGS[tag_name], nm.group(1))
                if key in onvif_names:
                    # This component collides with onvif.xsd — skip it entirely.
                    if is_self_closing:
                        i = tag_end
                    else:
                        # Use absolute positions in original body text via skip logic
                        # We need to skip past the closing tag for this element.
                        # Re-anchor to body and use the outer skip_element_subtree helper.
                        # We pass body[i:] and adjust.
                        sub_end = skip_element_subtree(body, i)
                        i = sub_end
                    # Swallow a single trailing newline (avoids blank lines in output)
                    if i < len(body) and body[i] == '\n':
                        i += 1
                    continue

        # Not a colliding global, or not a global component tag: emit as-is
        result.append(tag_text)
        i = tag_end

        # If this is a non-self-closing tag at the top level, emit everything up to
        # its matching close tag (the body is assumed flat, but nested elements exist).
        # We need to track depth to find where this top-level element ends.
        if not is_self_closing and tag_name is not None:
            depth = 1
            while i < len(body) and depth > 0:
                if body[i] != '<':
                    result.append(body[i])
                    i += 1
                    continue
                if body[i:i+4] == '<!--':
                    end = body.find('-->', i)
                    end_pos = end + 3 if end >= 0 else len(body)
                    result.append(body[i:end_pos])
                    i = end_pos
                    continue
                if body[i+1:i+2] == '/':
                    close_end = body.find('>', i)
                    close_end = close_end + 1 if close_end >= 0 else len(body)
                    tn_m = re.match(r'</([\w:]+)', body[i:])
                    result.append(body[i:close_end])
                    if tn_m and tn_m.group(1) == tag_name:
                        depth -= 1
                    i = close_end
                    continue
                inner_tag, inner_end, inner_self = _find_tag_end(body, i)
                result.append(inner_tag)
                if not inner_self:
                    inner_m = re.match(r'<([\w:]+)', inner_tag)
                    if inner_m and inner_m.group(1) == tag_name:
                        depth += 1
                i = inner_end

    return prefix + ''.join(result) + suffix + tail


def copy_shared_xsd(src_path, dst_path, onvif_names=None):
    """Copy a shared XSD with schemaLocation rewrites and determinism fixes.

    For common.xsd, also deduplicates against onvif.xsd: every top-level global
    component whose (symbol_space, name) appears in onvif_names is removed from
    the oracle's copy of common.xsd.  This is required because onvif.xsd v25.12
    contains <xs:include schemaLocation="common.xsd"/> and common.xsd v25.06
    (same targetNamespace) re-defines several types that now exist in onvif.xsd.
    Xerces (used by the oracle) rejects the duplicate within one target namespace;
    xmllint tolerated it.  The dedup keeps onvif.xsd's (newer, v25.12) definition
    and retains everything unique to common.xsd (PTZStatus, Vector2D, Color, etc.).
    """
    # wsn-b2.xsd gets a special enhanced version for the oracle
    basename = os.path.basename(dst_path)
    if basename == 'wsn-b2.xsd':
        with open(dst_path, 'w', encoding='utf-8') as f:
            f.write(WSN_B2_ENHANCED)
        print(f"Vendored (enhanced): {dst_path}")
        return

    with open(src_path, 'r', encoding='utf-8') as f:
        content = f.read()
    content = rewrite_schema_locations(content)
    # Fix non-deterministic content models: xs:any wildcards with namespace="##any" or
    # namespace="##targetNamespace" cause UPA (Unique Particle Attribution) violations
    # when they appear as sibling particles alongside explicitly-declared optional elements
    # in the same namespace.  Xerces (strict) rejects this; xmllint tolerated it.
    # Replace with '##other' so wildcards only match elements NOT in the schema's own
    # namespace (all elements in the schema's namespace are declared explicitly anyway).
    # This fix applies to both onvif.xsd and common.xsd shared schemas.
    #
    # EXCEPTION: ws-addr.xsd is excluded from the ##any→##other rewrite.
    # WS-Addressing ReferenceParametersType and MetadataType use ##any wildcards
    # legitimately — reference parameters may come from any namespace including the
    # addressing namespace itself.  The EndpointReferenceType content model uses named
    # optional elements (Address, ReferenceParameters, Metadata) followed by a ##other
    # trailing wildcard, which is already UPA-clean without any rewrite.  Rewriting
    # ##any→##other in ReferenceParametersType/MetadataType would cause Xerces to reject
    # addressing-namespace children (e.g. wsa:ReferenceParameters carrying a
    # wsa:SubscriptionId), which is the root cause of conformance finding A-1.
    if basename != 'ws-addr.xsd':
        content = content.replace(
            'namespace="##targetNamespace"',
            'namespace="##other"'
        )
        content = content.replace(
            'namespace="##any"',
            'namespace="##other"'
        )

    # Dedup common.xsd against onvif.xsd to satisfy Xerces' sch-props-correct.2 rule:
    # within one targetNamespace, every global component name must appear exactly once
    # across all xs:include-d documents.  Strip any common.xsd global whose name is
    # already defined in onvif.xsd; onvif.xsd's definition (v25.12) is authoritative.
    if basename == 'common.xsd' and onvif_names:
        before_count = len(re.findall(r'<xs:(?:complexType|simpleType|element|attribute|group|attributeGroup)\b', content))
        content = strip_colliding_globals(content, onvif_names)
        after_count = len(re.findall(r'<xs:(?:complexType|simpleType|element|attribute|group|attributeGroup)\b', content))
        stripped = before_count - after_count
        print(f"Deduped common.xsd: stripped {stripped} top-level component(s) that also appear in onvif.xsd")

    with open(dst_path, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f"Vendored: {dst_path}")


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <wsdl_dir> <output_dir>", file=sys.stderr)
        sys.exit(1)

    wsdl_dir = sys.argv[1]
    output_dir = sys.argv[2]
    os.makedirs(output_dir, exist_ok=True)

    # Extract body schemas from WSDLs
    services = [
        ("devicemgmt.wsdl", "device"),
        ("media.wsdl",      "media"),
        ("imaging.wsdl",    "imaging"),
        ("ptz.wsdl",        "ptz"),
        ("events.wsdl",     "events"),
    ]
    for wsdl_file, service_name in services:
        wsdl_path = os.path.join(wsdl_dir, wsdl_file)
        if not os.path.exists(wsdl_path):
            print(f"ERROR: missing {wsdl_path}", file=sys.stderr)
            sys.exit(1)
        extract_schemas_from_wsdl(wsdl_path, output_dir, service_name)

    # Collect top-level global component names from onvif.xsd so that common.xsd can be
    # deduplicated against them (Xerces rejects same-namespace duplicate global components
    # that xmllint tolerated).
    onvif_xsd_path = os.path.join(wsdl_dir, 'onvif.xsd')
    with open(onvif_xsd_path, 'r', encoding='utf-8') as f:
        onvif_text = f.read()
    onvif_names = collect_global_names(onvif_text)
    print(f"onvif.xsd: {len(onvif_names)} global component names collected for dedup")

    # Copy shared XSDs with schemaLocation rewrites
    for xsd_name in SHARED_XSDS:
        src = os.path.join(wsdl_dir, xsd_name)
        dst = os.path.join(output_dir, xsd_name)
        if not os.path.exists(src):
            print(f"ERROR: missing shared XSD {src}", file=sys.stderr)
            sys.exit(1)
        copy_shared_xsd(src, dst, onvif_names=onvif_names)

    print("Done.")


if __name__ == '__main__':
    main()
