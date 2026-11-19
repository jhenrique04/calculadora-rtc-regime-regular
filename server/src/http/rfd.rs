//! `Content-Disposition: inline;filename=f.txt` que o
//! `AbstractMessageConverterMethodProcessor` acrescenta contra RFD quando o
//! último segmento do caminho tem extensão "insegura"

use percent_encoding::percent_decode_str;

use super::resposta::Resposta;

pub const CABECALHO: &str = "inline;filename=f.txt";

// SAFE_EXTENSIONS + extensões cujo MIME (mime-mappings do Boot, senão o
// mime.types do Spring) é audio/, image/, video/ ou +xml; `yaml` vem do
// ContentNegotiationManager. Conferido extensão a extensão contra o Java
const SEGURAS: &[&str] = &[
    "3ds",
    "3g2",
    "3gp",
    "aac",
    "abs",
    "adp",
    "aif",
    "aifc",
    "aiff",
    "art",
    "asf",
    "asx",
    "atom",
    "atomcat",
    "atomsvc",
    "au",
    "avi",
    "avif",
    "avx",
    "axa",
    "axv",
    "bmp",
    "btif",
    "caf",
    "ccxml",
    "cdxml",
    "cgm",
    "cmx",
    "csv",
    "dae",
    "davmount",
    "dbk",
    "dd2",
    "dib",
    "djv",
    "djvu",
    "dra",
    "dtb",
    "dts",
    "dtshd",
    "dv",
    "dvb",
    "dwg",
    "dxf",
    "ecelp4800",
    "ecelp7470",
    "ecelp9600",
    "emma",
    "eol",
    "es3",
    "et3",
    "f4v",
    "fbs",
    "fh",
    "fh4",
    "fh5",
    "fh7",
    "fhc",
    "flac",
    "fli",
    "flv",
    "fpx",
    "fst",
    "fvt",
    "g3",
    "gif",
    "gml",
    "gpx",
    "grxml",
    "h261",
    "h263",
    "h264",
    "hal",
    "ico",
    "ief",
    "ink",
    "inkml",
    "irp",
    "jpe",
    "jpeg",
    "jpg",
    "jpgm",
    "jpgv",
    "jpm",
    "json",
    "jxl",
    "kar",
    "kml",
    "ktx",
    "lasxml",
    "lbe",
    "link66",
    "lostxml",
    "lvp",
    "m1v",
    "m2a",
    "m2t",
    "m2ts",
    "m2v",
    "m3a",
    "m3u",
    "m4a",
    "m4b",
    "m4r",
    "m4u",
    "m4v",
    "mac",
    "mads",
    "mathml",
    "mdi",
    "meta4",
    "metalink",
    "mets",
    "mid",
    "midi",
    "mj2",
    "mjp2",
    "mk3d",
    "mka",
    "mks",
    "mkv",
    "mmr",
    "mng",
    "mods",
    "mov",
    "movie",
    "mp1",
    "mp2",
    "mp2a",
    "mp3",
    "mp4",
    "mp4a",
    "mp4v",
    "mpa",
    "mpe",
    "mpeg",
    "mpega",
    "mpg",
    "mpg4",
    "mpga",
    "mpkg",
    "mpv2",
    "mrcx",
    "mscml",
    "mts",
    "musicxml",
    "mxml",
    "mxu",
    "ncx",
    "npx",
    "oga",
    "ogg",
    "ogv",
    "omdoc",
    "opf",
    "opus",
    "osfpvg",
    "pbm",
    "pct",
    "pcx",
    "pgm",
    "pic",
    "pict",
    "pls",
    "png",
    "pnm",
    "pnt",
    "ppm",
    "properties",
    "psd",
    "pskcxml",
    "pya",
    "pyv",
    "qt",
    "qti",
    "qtif",
    "ra",
    "ram",
    "ras",
    "rdf",
    "res",
    "rgb",
    "rif",
    "rip",
    "rl",
    "rlc",
    "rld",
    "rmi",
    "rmp",
    "rs",
    "rsd",
    "rss",
    "s3m",
    "sbml",
    "sdkd",
    "sdkm",
    "sgi",
    "shf",
    "sid",
    "sil",
    "smi",
    "smil",
    "smv",
    "snd",
    "spx",
    "sru",
    "srx",
    "ssdl",
    "ssml",
    "svg",
    "svgz",
    "tei",
    "teicorpus",
    "text",
    "tfi",
    "tga",
    "tif",
    "tiff",
    "ts",
    "txt",
    "ulw",
    "uoml",
    "uva",
    "uvg",
    "uvh",
    "uvi",
    "uvm",
    "uvp",
    "uvs",
    "uvt",
    "uvu",
    "uvv",
    "uvva",
    "uvvg",
    "uvvh",
    "uvvi",
    "uvvm",
    "uvvp",
    "uvvs",
    "uvvt",
    "uvvu",
    "uvvv",
    "viv",
    "vob",
    "vxml",
    "wav",
    "wax",
    "wbmp",
    "wbs",
    "wdp",
    "weba",
    "webm",
    "webp",
    "wm",
    "wma",
    "wmv",
    "wmx",
    "wsdl",
    "wspolicy",
    "wvx",
    "x3d",
    "x3dz",
    "xaml",
    "xbm",
    "xdf",
    "xdm",
    "xdp",
    "xdssc",
    "xenc",
    "xer",
    "xht",
    "xhtml",
    "xhvml",
    "xif",
    "xlf",
    "xm",
    "xml",
    "xop",
    "xpl",
    "xpm",
    "xslt",
    "xsm",
    "xspf",
    "xul",
    "xvm",
    "xvml",
    "xwd",
    "yaml",
    "yin",
    "yml",
    "zaz",
    "zmm",
];

fn extensao(nome: &str) -> Option<&str> {
    nome.rsplit_once('.')
        .map(|(_, e)| e)
        .filter(|e| !e.is_empty())
}

fn segura(extensao: Option<&str>) -> bool {
    extensao.is_none_or(|e| {
        SEGURAS
            .binary_search(&e.to_ascii_lowercase().as_str())
            .is_ok()
    })
}

/// status 1xx e 3xx ficam de fora, como no Spring
fn status_sujeito(r: &Resposta) -> bool {
    let s = r.status;
    !(s.is_informational() || s.is_redirection())
}

pub fn exige(caminho_bruto: &str, r: &Resposta) -> bool {
    if !r.conversor || !status_sujeito(r) {
        return false;
    }
    let ultimo = caminho_bruto.rsplit('/').next().unwrap_or_default();
    let (nome, parametros) = ultimo.split_at(ultimo.find(';').unwrap_or(ultimo.len()));
    let nome = percent_decode_str(nome).decode_utf8_lossy();
    let parametros = percent_decode_str(parametros).decode_utf8_lossy();
    !segura(extensao(&nome)) || !segura(extensao(&parametros))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::StatusCode;

    fn json(status: StatusCode) -> Resposta {
        Resposta::json(status, "{}".into())
    }

    #[test]
    fn tabela_ordenada_para_busca_binaria() {
        assert!(SEGURAS.windows(2).all(|p| p[0] < p[1]));
    }

    #[test]
    fn extensoes_como_no_java() {
        let r = json(StatusCode::NOT_FOUND);
        for inseguro in [
            "/api/x.css",
            "/api/x.JS",
            "/api/x.html",
            "/api/x.foo",
            "/api/a.css;x=1",
            "/api/%78.wasm",
        ] {
            assert!(exige(inseguro, &r), "{inseguro}");
        }
        for seguro in [
            "/api/x.yaml",
            "/api/x.svg",
            "/api/x.mp3",
            "/api/x.Json",
            "/api/x.",
            "/api/x",
            "/api/a.b/x",
        ] {
            assert!(!exige(seguro, &r), "{seguro}");
        }
    }

    #[test]
    fn so_respostas_de_conversor_fora_de_3xx() {
        assert!(!exige("/api/x.css", &json(StatusCode::FOUND)));
        assert!(!exige("/api/x.css", &Resposta::new(StatusCode::NOT_FOUND)));
        assert!(exige("/api/x.css", &json(StatusCode::OK)));
    }
}
