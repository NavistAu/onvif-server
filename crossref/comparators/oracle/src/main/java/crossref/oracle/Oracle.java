package crossref.oracle;

import com.sun.net.httpserver.HttpServer;
import org.apache.xml.security.Init;
import org.apache.xml.security.c14n.Canonicalizer;
import org.w3c.dom.ls.LSInput;
import org.w3c.dom.ls.LSResourceResolver;

import javax.xml.XMLConstants;
import javax.xml.transform.stream.StreamSource;
import javax.xml.validation.Schema;
import javax.xml.validation.SchemaFactory;
import javax.xml.validation.Validator;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;

/**
 * XML Schema oracle for ONVIF crossref conformance testing.
 *
 * <p>Exposes three HTTP endpoints:
 * <ul>
 *   <li>{@code /healthz}              — liveness probe (returns "ok")</li>
 *   <li>{@code /c14n}                 — POST XML body → Exclusive C14N bytes</li>
 *   <li>{@code /validate?schema=<id>} — POST XML fragment → JSON {valid, errors}</li>
 * </ul>
 *
 * <p>Registered schema ids:
 * <ul>
 *   <li>{@code soap12-envelope} — SOAP 1.2 envelope (W3C)</li>
 *   <li>{@code device-body}     — ONVIF Device Management service body elements</li>
 *   <li>{@code media-body}      — ONVIF Media service body elements</li>
 *   <li>{@code imaging-body}    — ONVIF Imaging service body elements</li>
 *   <li>{@code ptz-body}        — ONVIF PTZ service body elements</li>
 *   <li>{@code events-body}     — ONVIF Events service body elements</li>
 * </ul>
 *
 * <p>All schemas are bundled offline; no network access occurs at build or runtime.
 * Shared ONVIF schemas (onvif.xsd, common.xsd, ws-addr.xsd, soap-envelope.xsd,
 * xmlmime.xsd, xop-include.xsd, wsn-b2.xsd, wsn-t1.xsd) are resolved via
 * {@link OnvifSchemaResolver} entirely from classpath resources under /schemas/.
 */
public class Oracle {
    private static final Map<String, Schema> SCHEMAS = new HashMap<>();

    /**
     * LSResourceResolver that resolves schema imports from the classpath /schemas/ directory.
     *
     * <p>Maps both namespace URIs and systemId filenames to classpath resources so that
     * Xerces resolves ALL cross-schema imports offline. Every distinct {@code xs:import}
     * namespace across the ONVIF schema bundle is enumerated here.
     */
    private static class OnvifSchemaResolver implements LSResourceResolver {
        // Namespace URI -> classpath resource path
        private static final Map<String, String> NS_TO_RESOURCE = new HashMap<>();
        // systemId filename (bare name) -> classpath resource path
        private static final Map<String, String> FILENAME_TO_RESOURCE = new HashMap<>();

        static {
            // W3C XML namespace (needed by soap12-envelope.xsd)
            NS_TO_RESOURCE.put("http://www.w3.org/XML/1998/namespace", "/schemas/xml.xsd");
            // SOAP 1.2 envelope (needed by onvif.xsd and as a top-level schema id)
            NS_TO_RESOURCE.put("http://www.w3.org/2003/05/soap-envelope", "/schemas/soap-envelope.xsd");
            // SOAP 1.2 envelope full schema (same namespace)
            // onvif.xsd's soap-envelope import resolves here
            // WS-Addressing 2005/08 (needed by events-body.xsd)
            NS_TO_RESOURCE.put("http://www.w3.org/2005/08/addressing", "/schemas/ws-addr.xsd");
            // XMLMime (needed by onvif.xsd)
            NS_TO_RESOURCE.put("http://www.w3.org/2005/05/xmlmime", "/schemas/xmlmime.xsd");
            // XOP Include (needed by onvif.xsd)
            NS_TO_RESOURCE.put("http://www.w3.org/2004/08/xop/include", "/schemas/xop-include.xsd");
            // WS-BaseNotification b-2 (needed by onvif.xsd and events-body.xsd)
            NS_TO_RESOURCE.put("http://docs.oasis-open.org/wsn/b-2", "/schemas/wsn-b2.xsd");
            // WS-Topics t-1 (needed by events-body.xsd)
            NS_TO_RESOURCE.put("http://docs.oasis-open.org/wsn/t-1", "/schemas/wsn-t1.xsd");
            // ONVIF core schema (needed by all *-body schemas via onvif.xsd import)
            NS_TO_RESOURCE.put("http://www.onvif.org/ver10/schema", "/schemas/onvif.xsd");

            // Filename-based fallbacks (for schemaLocation bare filenames)
            FILENAME_TO_RESOURCE.put("xml.xsd",          "/schemas/xml.xsd");
            FILENAME_TO_RESOURCE.put("soap12-envelope.xsd",  "/schemas/soap12-envelope.xsd");
            FILENAME_TO_RESOURCE.put("soap-envelope.xsd", "/schemas/soap-envelope.xsd");
            FILENAME_TO_RESOURCE.put("ws-addr.xsd",       "/schemas/ws-addr.xsd");
            FILENAME_TO_RESOURCE.put("xmlmime.xsd",       "/schemas/xmlmime.xsd");
            FILENAME_TO_RESOURCE.put("xop-include.xsd",   "/schemas/xop-include.xsd");
            FILENAME_TO_RESOURCE.put("wsn-b2.xsd",        "/schemas/wsn-b2.xsd");
            FILENAME_TO_RESOURCE.put("wsn-t1.xsd",        "/schemas/wsn-t1.xsd");
            FILENAME_TO_RESOURCE.put("onvif.xsd",         "/schemas/onvif.xsd");
            FILENAME_TO_RESOURCE.put("common.xsd",        "/schemas/common.xsd");
        }

        @Override
        public LSInput resolveResource(String type, String namespaceURI, String publicId,
                                       String systemId, String baseURI) {
            String resource = null;

            // 1. Try systemId filename lookup FIRST (bare filename or path ending with filename).
            //    systemId takes priority because xs:include passes the including schema's
            //    targetNamespace as namespaceURI — if we checked namespace first we would
            //    incorrectly return onvif.xsd for the "common.xsd" xs:include inside onvif.xsd,
            //    causing Xerces to see every onvif.xsd definition twice (sch-props-correct.2).
            if (systemId != null) {
                // Try exact match first
                resource = FILENAME_TO_RESOURCE.get(systemId);
                if (resource == null) {
                    // Try just the filename portion
                    String filename = systemId;
                    int slash = filename.lastIndexOf('/');
                    if (slash >= 0) filename = filename.substring(slash + 1);
                    resource = FILENAME_TO_RESOURCE.get(filename);
                }
            }

            // 2. Fall back to namespace URI lookup (handles xs:import with no schemaLocation)
            if (resource == null && namespaceURI != null) {
                resource = NS_TO_RESOURCE.get(namespaceURI);
            }

            if (resource == null) return null;

            final String res = resource;
            return new LSInput() {
                public java.io.Reader getCharacterStream()        { return null; }
                public void setCharacterStream(java.io.Reader r)  {}
                public InputStream getByteStream()                { return Oracle.class.getResourceAsStream(res); }
                public void setByteStream(InputStream i)          {}
                public String getStringData()                     { return null; }
                public void setStringData(String s)               {}
                public String getSystemId()                       { return systemId; }
                public void setSystemId(String s)                 {}
                public String getPublicId()                       { return publicId; }
                public void setPublicId(String s)                 {}
                public String getBaseURI()                        { return baseURI; }
                public void setBaseURI(String s)                  {}
                public String getEncoding()                       { return "UTF-8"; }
                public void setEncoding(String s)                 {}
                public boolean getCertifiedText()                 { return false; }
                public void setCertifiedText(boolean b)           {}
            };
        }
    }

    public static void main(String[] args) throws Exception {
        Init.init(); // Santuario initialisation

        SchemaFactory sf = SchemaFactory.newInstance(XMLConstants.W3C_XML_SCHEMA_NS_URI);
        sf.setResourceResolver(new OnvifSchemaResolver());

        // Register all schema ids. Shared ONVIF schemas (onvif.xsd, common.xsd, etc.)
        // are imported transitively and resolved by OnvifSchemaResolver; they are not
        // registered as top-level ids because the oracle validates body children (not
        // full envelopes) against the service-specific body schema.
        register(sf, "soap12-envelope", "/schemas/soap12-envelope.xsd");
        register(sf, "device-body",     "/schemas/device-body.xsd");
        register(sf, "media-body",      "/schemas/media-body.xsd");
        register(sf, "imaging-body",    "/schemas/imaging-body.xsd");
        register(sf, "ptz-body",        "/schemas/ptz-body.xsd");
        register(sf, "events-body",     "/schemas/events-body.xsd");

        HttpServer server = HttpServer.create(new InetSocketAddress("0.0.0.0", 8081), 0);
        server.createContext("/healthz",  ex -> respond(ex, 200, "ok".getBytes()));
        server.createContext("/c14n",     Oracle::handleC14n);
        server.createContext("/validate", Oracle::handleValidate);
        server.setExecutor(null);
        System.err.println("oracle listening on 0.0.0.0:8081");
        server.start();
    }

    private static void register(SchemaFactory sf, String id, String resource) throws Exception {
        try (InputStream in = Oracle.class.getResourceAsStream(resource)) {
            if (in == null) throw new IllegalStateException("missing schema resource: " + resource);
            SCHEMAS.put(id, sf.newSchema(new StreamSource(in)));
        }
    }

    private static void handleC14n(com.sun.net.httpserver.HttpExchange ex) {
        try {
            byte[] body = ex.getRequestBody().readAllBytes();
            Canonicalizer c = Canonicalizer.getInstance(
                    Canonicalizer.ALGO_ID_C14N_EXCL_OMIT_COMMENTS);
            ByteArrayOutputStream out = new ByteArrayOutputStream();
            c.canonicalize(body, out, false);
            respond(ex, 200, out.toByteArray());
        } catch (Exception e) {
            respond(ex, 400,
                    ("c14n error: " + e.getMessage()).getBytes(StandardCharsets.UTF_8));
        }
    }

    private static void handleValidate(com.sun.net.httpserver.HttpExchange ex) {
        try {
            String q = ex.getRequestURI().getQuery(); // schema=<id>
            String id = q != null && q.startsWith("schema=") ? q.substring(7) : "";
            Schema schema = SCHEMAS.get(id);
            byte[] body = ex.getRequestBody().readAllBytes();
            if (schema == null) {
                respond(ex, 200,
                        ("{\"valid\":false,\"errors\":[\"unknown schema id: " + id + "\"]}")
                            .getBytes(StandardCharsets.UTF_8));
                return;
            }
            Validator v = schema.newValidator();
            // Ensure no network access during validation (belt-and-suspenders)
            v.setResourceResolver(new OnvifSchemaResolver());
            final StringBuilder errs = new StringBuilder();
            v.setErrorHandler(new org.xml.sax.ErrorHandler() {
                public void warning(org.xml.sax.SAXParseException e)   {}
                public void error(org.xml.sax.SAXParseException e)     { errs.append(e.getMessage()).append("|"); }
                public void fatalError(org.xml.sax.SAXParseException e){ errs.append(e.getMessage()).append("|"); }
            });
            try {
                v.validate(new StreamSource(new ByteArrayInputStream(body)));
            } catch (Exception e) {
                if (errs.length() == 0) errs.append(e.getMessage());
            }
            String json = errs.length() == 0
                ? "{\"valid\":true}"
                : "{\"valid\":false,\"errors\":[\"" +
                    errs.toString().replace("\"", "'").replace("\n", " ") + "\"]}";
            respond(ex, 200, json.getBytes(StandardCharsets.UTF_8));
        } catch (Exception e) {
            respond(ex, 500,
                    ("{\"valid\":false,\"errors\":[\"oracle error\"]}").getBytes(StandardCharsets.UTF_8));
        }
    }

    private static void respond(com.sun.net.httpserver.HttpExchange ex, int code, byte[] body) {
        try (OutputStream os = ex.getResponseBody()) {
            ex.sendResponseHeaders(code, body.length);
            os.write(body);
        } catch (Exception ignored) {}
    }
}
