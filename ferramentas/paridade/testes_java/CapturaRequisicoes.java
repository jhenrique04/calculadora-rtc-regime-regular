package br.gov.serpro.rtc;

import java.io.ByteArrayInputStream;
import java.io.FileWriter;
import java.io.IOException;
import java.util.Collections;
import java.util.HexFormat;

import org.springframework.core.Ordered;
import org.springframework.core.annotation.Order;
import org.springframework.stereotype.Component;
import org.springframework.web.filter.OncePerRequestFilter;

import jakarta.servlet.FilterChain;
import jakarta.servlet.ReadListener;
import jakarta.servlet.ServletException;
import jakarta.servlet.ServletInputStream;
import jakarta.servlet.http.HttpServletRequest;
import jakarta.servlet.http.HttpServletRequestWrapper;
import jakarta.servlet.http.HttpServletResponse;

@Component
@Order(Ordered.HIGHEST_PRECEDENCE)
public class CapturaRequisicoes extends OncePerRequestFilter {

    private static String json(String s) {
        if (s == null) return "null";
        StringBuilder b = new StringBuilder("\"");
        for (char c : s.toCharArray()) {
            if (c == '"' || c == '\\') b.append('\\').append(c);
            else if (c < 0x20 || c > 0x7e) b.append(String.format("\\u%04x", (int) c));
            else b.append(c);
        }
        return b.append('"').toString();
    }

    @Override
    protected void doFilterInternal(HttpServletRequest req, HttpServletResponse res, FilterChain chain)
            throws ServletException, IOException {
        byte[] corpo = req.getInputStream().readAllBytes();
        StringBuilder h = new StringBuilder("[");
        for (String nome : Collections.list(req.getHeaderNames())) {
            for (String v : Collections.list(req.getHeaders(nome))) {
                if (h.length() > 1) h.append(',');
                h.append('[').append(json(nome)).append(',').append(json(v)).append(']');
            }
        }
        h.append(']');
        String teste = System.getProperty("captura.teste", "");
        StackTraceElement origem = null;
        for (StackTraceElement e : Thread.currentThread().getStackTrace()) {
            if (e.getClassName().startsWith("br.gov.serpro.rtc") && !e.getClassName().contains("CapturaRequisicoes")
                    && (e.getClassName().contains("Test") || e.getClassName().contains("Teste"))) {
                origem = e;
                break;
            }
        }
        String linha = "{\"teste\":" + json(origem == null ? teste : origem.getClassName() + "." + origem.getMethodName())
                + ",\"metodo\":" + json(req.getMethod())
                + ",\"uri\":" + json(req.getRequestURI())
                + ",\"contexto\":" + json(req.getContextPath())
                + ",\"consulta\":" + json(req.getQueryString())
                + ",\"cabecalhos\":" + h
                + ",\"corpo_hex\":" + json(HexFormat.of().formatHex(corpo))
                + ",\"tem_corpo\":" + (corpo.length > 0 || req.getContentType() != null) + "}\n";
        synchronized (CapturaRequisicoes.class) {
            try (FileWriter w = new FileWriter("/app/captura.jsonl", true)) {
                w.write(linha);
            }
        }
        HttpServletRequest copia = new HttpServletRequestWrapper(req) {
            @Override
            public ServletInputStream getInputStream() {
                ByteArrayInputStream in = new ByteArrayInputStream(corpo);
                return new ServletInputStream() {
                    public boolean isFinished() { return in.available() == 0; }
                    public boolean isReady() { return true; }
                    public void setReadListener(ReadListener l) { }
                    public int read() { return in.read(); }
                };
            }
        };
        chain.doFilter(copia, res);
    }
}
