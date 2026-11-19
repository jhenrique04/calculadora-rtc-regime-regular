# syntax=docker/dockerfile:1
FROM rtc-java-ref
COPY CapturaRequisicoes.java /app/src/test/java/br/gov/serpro/rtc/CapturaRequisicoes.java
RUN --mount=type=cache,target=/root/.m2 cd /app && rm -f captura.jsonl \
 && (./mvnw -B test -Dsurefire.failIfNoSpecifiedTests=false -DfailIfNoTests=false -Dmaven.test.failure.ignore=true > /app/testes.log 2>&1; echo "mvn saiu com $?" >> /app/testes.log)
