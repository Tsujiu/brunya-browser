use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchEngineDef {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub query_template: &'static str,
    pub glyph: &'static str,
}

pub static ALL_SEARCH_ENGINES: &[SearchEngineDef] = &[
    // Motor Principal Recomendado por Padrão
    SearchEngineDef { id: "startpage", name: "Startpage", category: "Privacidade", query_template: "https://www.startpage.com/sp/search?query=%s", glyph: "SP" },
    SearchEngineDef { id: "google", name: "Google", category: "Geral", query_template: "https://www.google.com/search?q=%s&hl=pt-BR", glyph: "G" },
    SearchEngineDef { id: "duckduckgo", name: "DuckDuckGo", category: "Privacidade", query_template: "https://duckduckgo.com/?q=%s", glyph: "🦆" },
    SearchEngineDef { id: "bing", name: "Microsoft Bing", category: "Geral", query_template: "https://www.bing.com/search?q=%s", glyph: "b" },
    SearchEngineDef { id: "brave", name: "Brave Search", category: "Privacidade", query_template: "https://search.brave.com/search?q=%s", glyph: "🦁" },
    SearchEngineDef { id: "yahoo", name: "Yahoo! Search", category: "Geral", query_template: "https://search.yahoo.com/search?p=%s", glyph: "Y!" },
    SearchEngineDef { id: "qwant", name: "Qwant", category: "Privacidade", query_template: "https://www.qwant.com/?q=%s", glyph: "Q" },
    SearchEngineDef { id: "mojeek", name: "Mojeek", category: "Privacidade", query_template: "https://www.mojeek.com/search?q=%s", glyph: "M" },
    SearchEngineDef { id: "swisscows", name: "Swisscows", category: "Privacidade", query_template: "https://swisscows.com/web?query=%s", glyph: "🐮" },
    SearchEngineDef { id: "searxng", name: "SearXNG", category: "Privacidade", query_template: "https://searx.space/search?q=%s", glyph: "🔍" },
    SearchEngineDef { id: "metager", name: "Metager", category: "Privacidade", query_template: "https://metager.org/meta/meta.ger3?eingabe=%s", glyph: "MG" },
    SearchEngineDef { id: "gibiru", name: "Gibiru", category: "Privacidade", query_template: "https://gibiru.com/results.html?q=%s", glyph: "Gi" },
    SearchEngineDef { id: "kagi", name: "Kagi", category: "Privacidade", query_template: "https://kagi.com/search?q=%s", glyph: "K" },
    SearchEngineDef { id: "you", name: "You.com", category: "Privacidade", query_template: "https://you.com/search?q=%s", glyph: "Y" },
    SearchEngineDef { id: "presearch", name: "Presearch", category: "Privacidade", query_template: "https://presearch.com/search?q=%s", glyph: "P" },

    // Ecológicos e Alternativos
    SearchEngineDef { id: "ecosia", name: "Ecosia", category: "Ecológico", query_template: "https://www.ecosia.org/search?q=%s", glyph: "🌱" },
    SearchEngineDef { id: "lilo", name: "Lilo", category: "Ecológico", query_template: "https://search.lilo.org/?q=%s", glyph: "💧" },
    SearchEngineDef { id: "oceanhero", name: "OceanHero", category: "Ecológico", query_template: "https://oceanhero.today/web?q=%s", glyph: "🌊" },
    SearchEngineDef { id: "ekoru", name: "Ekoru", category: "Ecológico", query_template: "https://ekoru.org/?q=%s", glyph: "🍃" },

    // Regionais / Internacionais
    SearchEngineDef { id: "baidu", name: "Baidu (China)", category: "Regional", query_template: "https://www.baidu.com/s?wd=%s", glyph: "百度" },
    SearchEngineDef { id: "yandex", name: "Yandex (Rússia)", category: "Regional", query_template: "https://yandex.com/search/?text=%s", glyph: "Я" },
    SearchEngineDef { id: "naver", name: "Naver (Coreia do Sul)", category: "Regional", query_template: "https://search.naver.com/search.naver?query=%s", glyph: "N" },
    SearchEngineDef { id: "daum", name: "Daum (Coreia do Sul)", category: "Regional", query_template: "https://search.daum.net/search?q=%s", glyph: "Da" },
    SearchEngineDef { id: "seznam", name: "Seznam.cz (Rep. Tcheca)", category: "Regional", query_template: "https://search.seznam.cz/?q=%s", glyph: "Sz" },
    SearchEngineDef { id: "sogou", name: "Sogou (China)", category: "Regional", query_template: "https://www.sogou.com/web?query=%s", glyph: "搜" },
    SearchEngineDef { id: "so360", name: "360 Search / SO (China)", category: "Regional", query_template: "https://www.so.com/s?q=%s", glyph: "360" },
    SearchEngineDef { id: "shenma", name: "Shenma (China)", category: "Regional", query_template: "https://m.sm.cn/s?q=%s", glyph: "神" },
    SearchEngineDef { id: "petal", name: "Petal Search (Huawei)", category: "Regional", query_template: "https://petalsearch.com/search?query=%s", glyph: "🌸" },
    SearchEngineDef { id: "goo", name: "Goo (Japão)", category: "Regional", query_template: "https://search.goo.ne.jp/web.jsp?MT=%s", glyph: "goo" },
    SearchEngineDef { id: "excite_jp", name: "Excite Japan", category: "Regional", query_template: "https://search.excite.co.jp/search.phtml?q=%s", glyph: "Ex" },
    SearchEngineDef { id: "yahoo_jp", name: "Yahoo! Japan", category: "Regional", query_template: "https://search.yahoo.co.jp/search?p=%s", glyph: "YJ" },
    SearchEngineDef { id: "rambler", name: "Rambler (Rússia)", category: "Regional", query_template: "https://nova.rambler.ru/search?query=%s", glyph: "Ra" },
    SearchEngineDef { id: "mailru", name: "Mail.ru Search", category: "Regional", query_template: "https://go.mail.ru/search?q=%s", glyph: "@" },
    SearchEngineDef { id: "exalead", name: "Exalead (França)", category: "Regional", query_template: "https://www.exalead.com/search/web/results/?q=%s", glyph: "Ex" },

    // Web Independente / Retrô / Experimental
    SearchEngineDef { id: "marginalia", name: "Marginalia Search", category: "Independente", query_template: "https://search.marginalia.nu/search?query=%s", glyph: "📜" },
    SearchEngineDef { id: "wiby", name: "Wiby (Web Retrô)", category: "Independente", query_template: "https://wiby.me/?q=%s", glyph: "🕹️" },
    SearchEngineDef { id: "millionshort", name: "Million Short", category: "Independente", query_template: "https://millionshort.com/search?keyword=%s", glyph: "✂️" },
    SearchEngineDef { id: "gigablast", name: "Gigablast", category: "Independente", query_template: "https://gigablast.com/search?q=%s", glyph: "GB" },
    SearchEngineDef { id: "yep", name: "Yep (Ahrefs)", category: "Independente", query_template: "https://yep.com/web?q=%s", glyph: "Y!" },
    SearchEngineDef { id: "dogpile", name: "Dogpile", category: "Metamotor", query_template: "https://www.dogpile.com/serp?q=%s", glyph: "🐕" },
    SearchEngineDef { id: "webcrawler", name: "WebCrawler", category: "Metamotor", query_template: "https://www.webcrawler.com/serp?q=%s", glyph: "🕷️" },
    SearchEngineDef { id: "info", name: "Info.com", category: "Metamotor", query_template: "https://www.info.com/serp?q=%s", glyph: "ℹ️" },
    SearchEngineDef { id: "zapmeta", name: "ZapMeta", category: "Metamotor", query_template: "https://www.zapmeta.com/search?q=%s", glyph: "⚡" },
    SearchEngineDef { id: "ask", name: "Ask.com", category: "Clássico", query_template: "https://www.ask.com/web?q=%s", glyph: "A" },
    SearchEngineDef { id: "aol", name: "AOL Search", category: "Clássico", query_template: "https://search.aol.com/aol/search?q=%s", glyph: "AOL" },
    SearchEngineDef { id: "lycos", name: "Lycos", category: "Clássico", query_template: "https://search.lycos.com/web/?q=%s", glyph: "🐕" },
    SearchEngineDef { id: "excite", name: "Excite", category: "Clássico", query_template: "https://results.excite.com/serp?q=%s", glyph: "!" },

    // Conhecimento Computacional e Cibersegurança / Infraestrutura
    SearchEngineDef { id: "wolfram", name: "Wolfram|Alpha", category: "Cálculo/IA", query_template: "https://www.wolframalpha.com/input?i=%s", glyph: "∫" },
    SearchEngineDef { id: "shodan", name: "Shodan (Infra/IoT)", category: "Segurança", query_template: "https://www.shodan.io/search?query=%s", glyph: "⚡" },
    SearchEngineDef { id: "censys", name: "Censys (Segurança Web)", category: "Segurança", query_template: "https://search.censys.io/hosts?q=%s", glyph: "🛡️" },
    SearchEngineDef { id: "zoomeye", name: "ZoomEye (Ciberespaço)", category: "Segurança", query_template: "https://www.zoomeye.org/searchResult?q=%s", glyph: "👁️" },
    SearchEngineDef { id: "fofa", name: "FOFA (Ativos Ciber)", category: "Segurança", query_template: "https://fofa.info/result?qbase64=%s", glyph: "🌐" },
    SearchEngineDef { id: "hunter", name: "Hunter.how", category: "Segurança", query_template: "https://hunter.how/list?search=%s", glyph: "🎯" },
    SearchEngineDef { id: "criminalip", name: "Criminal IP", category: "Segurança", query_template: "https://www.criminalip.io/asset/search?query=%s", glyph: "🚨" },

    // Acadêmico e Código
    SearchEngineDef { id: "scholar", name: "Google Scholar / Acadêmico", category: "Acadêmico", query_template: "https://scholar.google.com/scholar?q=%s", glyph: "🎓" },
    SearchEngineDef { id: "semantic", name: "Semantic Scholar", category: "Acadêmico", query_template: "https://www.semanticscholar.org/search?q=%s", glyph: "📖" },
    SearchEngineDef { id: "base", name: "BASE (Bielefeld Academic)", category: "Acadêmico", query_template: "https://www.base-search.net/Search/Results?lookfor=%s", glyph: "🏛️" },
    SearchEngineDef { id: "core", name: "CORE (Repositórios Abertos)", category: "Acadêmico", query_template: "https://core.ac.uk/search?q=%s", glyph: "📚" },
    SearchEngineDef { id: "archive", name: "Internet Archive / Wayback", category: "Arquivo", query_template: "https://archive.org/search?query=%s", glyph: "🏛️" },
    SearchEngineDef { id: "sourcegraph", name: "Sourcegraph (Código Aberto)", category: "Código", query_template: "https://sourcegraph.com/search?q=%s", glyph: "💻" },
    SearchEngineDef { id: "grepapp", name: "Grep.app (Código GitHub)", category: "Código", query_template: "https://grep.app/search?q=%s", glyph: "🔎" },
    SearchEngineDef { id: "publicwww", name: "PublicWWW (Código Web)", category: "Código", query_template: "https://publicwww.com/websites/%s", glyph: "💻" },

    // Imagens e Mídia
    SearchEngineDef { id: "tineye", name: "TinEye (Busca Reversa)", category: "Imagens", query_template: "https://tineye.com/search?url=%s", glyph: "🖼️" },
    SearchEngineDef { id: "yandex_img", name: "Yandex Images (Reversa)", category: "Imagens", query_template: "https://yandex.com/images/search?text=%s", glyph: "🖼️" },
    SearchEngineDef { id: "giphy", name: "Giphy (Busca GIFs)", category: "Mídia", query_template: "https://giphy.com/search/%s", glyph: "🎞️" },
    SearchEngineDef { id: "tenor", name: "Tenor (Busca GIFs)", category: "Mídia", query_template: "https://tenor.com/search/%s-gifs", glyph: "🎬" },
];

pub fn get_engine_by_id(id: &str) -> Option<&'static SearchEngineDef> {
    ALL_SEARCH_ENGINES.iter().find(|e| e.id == id)
}
