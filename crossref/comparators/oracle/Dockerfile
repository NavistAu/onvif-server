# build stage — schema extraction (python3) + Java compilation (maven)
#
# NOTE: the build context is the onvif-server REPO ROOT (compose sets
# `context: ..` + `dockerfile: crossref/comparators/oracle/Dockerfile`), so the
# extraction reads the CANONICAL top-level `wsdl/` — single source of truth, so
# any WSDL change (e.g. the PTZ Stop fix) flows through with no vendored copy to
# drift. Non-wsdl COPYs are therefore qualified with the oracle subpath.
FROM maven:3.9-eclipse-temurin-21@sha256:1bb51c5ed28b95aef2bc7b46bff6940da43747cdaf838ce4afc2081ce9403750 AS build
WORKDIR /src

# The maven/temurin image is Java-only; the schema-extraction step needs python3.
RUN apt-get update \
    && apt-get install -y --no-install-recommends python3 \
    && rm -rf /var/lib/apt/lists/*

# Cache Maven dependencies first
COPY crossref/comparators/oracle/pom.xml .
RUN mvn -q dependency:go-offline

# Copy source (pre-committed schemas + Java source)
COPY crossref/comparators/oracle/src ./src

# Copy the CANONICAL ONVIF WSDLs + shared XSDs from the repo's top-level wsdl/.
# The extraction script re-generates the *-body.xsd and vendored shared XSDs,
# overwriting any pre-committed versions with the freshest extraction.
COPY wsdl/ /wsdl/
COPY crossref/comparators/oracle/extract-schemas.py /extract-schemas.py
RUN python3 /extract-schemas.py /wsdl src/main/resources/schemas

RUN mvn -q package -DskipTests

# runtime stage — only the JRE + shaded jar
FROM eclipse-temurin:21-jre@sha256:010e0a06bd4e0184dec58626afb3ba727b42c56c91b977e2f0a9e0837e0fa3fb
WORKDIR /app
COPY --from=build /src/target/oracle.jar /app/oracle.jar
EXPOSE 8081
ENTRYPOINT ["java", "-jar", "/app/oracle.jar"]
