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


def copy_shared_xsd(src_path, dst_path):
    """Copy a shared XSD with schemaLocation rewrites and determinism fixes."""
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
    # Fix non-deterministic content models: 'namespace="##targetNamespace"' on xs:any
    # causes ambiguity with explicitly-declared sibling elements in the same namespace.
    # Replace with '##other' so the wildcard only matches elements NOT in the schema's
    # own namespace (which are all declared explicitly anyway).
    # This is required for xmllint/strict validators; Xerces also accepts it.
    content = content.replace(
        'namespace="##targetNamespace"',
        'namespace="##other"'
    )
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

    # Copy shared XSDs with schemaLocation rewrites
    for xsd_name in SHARED_XSDS:
        src = os.path.join(wsdl_dir, xsd_name)
        dst = os.path.join(output_dir, xsd_name)
        if not os.path.exists(src):
            print(f"ERROR: missing shared XSD {src}", file=sys.stderr)
            sys.exit(1)
        copy_shared_xsd(src, dst)

    print("Done.")


if __name__ == '__main__':
    main()
