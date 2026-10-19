//! Normalização de consultas e remoção de diacríticos para fallback de busca.

/// Normaliza uma consulta do usuário: remove espaços das bordas, colapsa
/// espaços internos em um só e converte para minúsculas.
pub fn normalize_query(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Remove diacríticos (acentos, cedilha, til etc.) decompondo os caracteres
/// em NFD e descartando marcas combinantes. Letras que não decompõem em base +
/// marca (como `ß`, `ø`, `æ`) são mapeadas explicitamente para a forma ASCII
/// mais próxima.
pub fn strip_diacritics(input: &str) -> String {
    input
        .chars()
        .flat_map(|c| match c {
            'ß' => "ss".chars().collect::<Vec<_>>(),
            'æ' | 'Æ' => vec!['a', 'e'],
            'œ' | 'Œ' => vec!['o', 'e'],
            'ø' | 'Ø' => vec!['o'],
            'đ' | 'Đ' => vec!['d'],
            'ð' | 'Ð' => vec!['d'],
            'þ' | 'Þ' => vec!['t', 'h'],
            'ł' | 'Ł' => vec!['l'],
            _ => decompose(c),
        })
        .filter(|c| !is_combining_mark(*c))
        .collect()
}

/// Decomposição canônica (NFD) manual dos precompostos latinos comuns.
/// Implementada sem dependência externa de normalização Unicode.
fn decompose(c: char) -> Vec<char> {
    match c {
        'à'..='å' | 'ā' | 'ă' | 'ą' => vec!['a', combining_of(c)],
        'À'..='Å' | 'Ā' | 'Ă' | 'Ą' => vec!['A', combining_of(c)],
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => vec!['c', combining_of(c)],
        'Ç' | 'Ć' | 'Ĉ' | 'Ċ' | 'Č' => vec!['C', combining_of(c)],
        'ď' | 'ḍ' => vec!['d', combining_of(c)],
        'Ď' | 'Ḍ' => vec!['D', combining_of(c)],
        'è'..='ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => vec!['e', combining_of(c)],
        'È'..='Ë' | 'Ē' | 'Ĕ' | 'Ė' | 'Ę' | 'Ě' => vec!['E', combining_of(c)],
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => vec!['g', combining_of(c)],
        'Ĝ' | 'Ğ' | 'Ġ' | 'Ģ' => vec!['G', combining_of(c)],
        'ĥ' | 'ħ' => vec!['h', combining_of(c)],
        'Ĥ' | 'Ħ' => vec!['H', combining_of(c)],
        'ì'..='ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => vec!['i', combining_of(c)],
        'Ì'..='Ï' | 'Ĩ' | 'Ī' | 'Ĭ' | 'Į' | 'İ' => vec!['I', combining_of(c)],
        'ĵ' => vec!['j', '\u{0302}'],
        'Ĵ' => vec!['J', '\u{0302}'],
        'ķ' => vec!['k', '\u{0327}'],
        'Ķ' => vec!['K', '\u{0327}'],
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' => vec!['l', combining_of(c)],
        'Ĺ' | 'Ļ' | 'Ľ' | 'Ŀ' => vec!['L', combining_of(c)],
        'ñ' | 'ń' | 'ņ' | 'ň' => vec!['n', combining_of(c)],
        'Ñ' | 'Ń' | 'Ņ' | 'Ň' => vec!['N', combining_of(c)],
        'ò'..='ö' | 'ō' | 'ŏ' | 'ő' => vec!['o', combining_of(c)],
        'Ò'..='Ö' | 'Ō' | 'Ŏ' | 'Ő' => vec!['O', combining_of(c)],
        'ŕ' | 'ŗ' | 'ř' => vec!['r', combining_of(c)],
        'Ŕ' | 'Ŗ' | 'Ř' => vec!['R', combining_of(c)],
        'ś' | 'ŝ' | 'ş' | 'š' => vec!['s', combining_of(c)],
        'Ś' | 'Ŝ' | 'Ş' | 'Š' => vec!['S', combining_of(c)],
        'ţ' | 'ť' | 'ŧ' => vec!['t', combining_of(c)],
        'Ţ' | 'Ť' | 'Ŧ' => vec!['T', combining_of(c)],
        'ù'..='ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => vec!['u', combining_of(c)],
        'Ù'..='Ü' | 'Ũ' | 'Ū' | 'Ŭ' | 'Ů' | 'Ű' | 'Ų' => vec!['U', combining_of(c)],
        'ŵ' => vec!['w', '\u{0302}'],
        'Ŵ' => vec!['W', '\u{0302}'],
        'ý' | 'ÿ' | 'ŷ' => vec!['y', combining_of(c)],
        'Ý' | 'Ŷ' | 'Ÿ' => vec!['Y', combining_of(c)],
        'ź' | 'ż' | 'ž' => vec!['z', combining_of(c)],
        'Ź' | 'Ż' | 'Ž' => vec!['Z', combining_of(c)],
        _ => vec![c],
    }
}

/// Marca combinante (diacrítico) associada a um caractere precomposto.
fn combining_of(c: char) -> char {
    match c {
        // agudo
        'á' | 'Á' | 'ć' | 'Ć' | 'é' | 'É' | 'í' | 'Í' | 'ĺ' | 'Ĺ' | 'ń' | 'Ń' | 'ó' | 'Ó'
        | 'ŕ' | 'Ŕ' | 'ś' | 'Ś' | 'ú' | 'Ú' | 'ý' | 'Ý' | 'ź' | 'Ź' => '\u{0301}',
        // grave
        'à' | 'À' | 'è' | 'È' | 'ì' | 'Ì' | 'ò' | 'Ò' | 'ù' | 'Ù' => '\u{0300}',
        // circunflexo
        'â' | 'Â' | 'ĉ' | 'Ĉ' | 'ê' | 'Ê' | 'ĝ' | 'Ĝ' | 'î' | 'Î' | 'ô' | 'Ô' | 'ŝ' | 'Ŝ'
        | 'û' | 'Û' | 'ŷ' | 'Ŷ' => '\u{0302}',
        // til
        'ã' | 'Ã' | 'ĩ' | 'Ĩ' | 'ñ' | 'Ñ' | 'õ' | 'Õ' | 'ũ' | 'Ũ' => '\u{0303}',
        // trema
        'ä' | 'Ä' | 'ë' | 'Ë' | 'ï' | 'Ï' | 'ö' | 'Ö' | 'ü' | 'Ü' | 'ÿ' | 'Ÿ' => '\u{0308}',
        // anel
        'å' | 'Å' | 'ů' | 'Ů' => '\u{030A}',
        // cedilha
        'ç' | 'Ç' | 'ģ' | 'Ģ' | 'ķ' | 'Ķ' | 'ļ' | 'Ļ' | 'ņ' | 'Ņ' | 'ŗ' | 'Ŗ' | 'ş' | 'Ş'
        | 'ţ' | 'Ţ' => '\u{0327}',
        // macron
        'ā' | 'Ā' | 'ē' | 'Ē' | 'ī' | 'Ī' | 'ō' | 'Ō' | 'ū' | 'Ū' => '\u{0304}',
        // breve
        'ă' | 'Ă' | 'ĕ' | 'Ĕ' | 'ğ' | 'Ğ' | 'ĭ' | 'Ĭ' | 'ŏ' | 'Ŏ' | 'ŭ' | 'Ŭ' => '\u{0306}',
        // ogonek
        'ą' | 'Ą' | 'ę' | 'Ę' | 'į' | 'Į' | 'ų' | 'Ų' => '\u{0328}',
        // ponto acima
        'ċ' | 'Ċ' | 'ė' | 'Ė' | 'ġ' | 'Ġ' | 'ż' | 'Ż' => '\u{0307}',
        // caron
        'č' | 'Č' | 'ď' | 'Ď' | 'ě' | 'Ě' | 'ľ' | 'Ľ' | 'ň' | 'Ň' | 'ř' | 'Ř' | 'š' | 'Š'
        | 'ť' | 'Ť' | 'ž' | 'Ž' => '\u{030C}',
        // acento duplo agudo
        'ő' | 'Ő' | 'ű' | 'Ű' => '\u{030B}',
        // apóstrofo / ponto mediano
        'ŀ' | 'Ŀ' => '\u{00B7}',
        // barra atravessada
        'ħ' | 'Ħ' | 'ŧ' | 'Ŧ' => '\u{0335}',
        // ponto abaixo
        'ḍ' | 'Ḍ' => '\u{0323}',
        _ => '\u{0301}',
    }
}

fn is_combining_mark(c: char) -> bool {
    ('\u{0300}'..='\u{036F}').contains(&c) || c == '\u{00B7}'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_trim_lowercase_e_espacos() {
        assert_eq!(normalize_query("  Hello   World  "), "hello world");
        assert_eq!(normalize_query("RUN"), "run");
        assert_eq!(normalize_query("  Café  "), "café");
        assert_eq!(normalize_query(""), "");
        assert_eq!(normalize_query("a\tb\nc"), "a b c");
    }

    #[test]
    fn strip_diacritics_portugues() {
        assert_eq!(strip_diacritics("ção"), "cao");
        assert_eq!(strip_diacritics("maçã"), "maca");
        assert_eq!(strip_diacritics("café"), "cafe");
        assert_eq!(strip_diacritics("pêssego"), "pessego");
        assert_eq!(strip_diacritics("Coração"), "Coracao");
        assert_eq!(strip_diacritics("Árvore"), "Arvore");
        assert_eq!(strip_diacritics("língua"), "lingua");
        assert_eq!(strip_diacritics("inglês"), "ingles");
    }

    #[test]
    fn strip_diacritics_sem_diacriticos_permance_igual() {
        assert_eq!(strip_diacritics("apple"), "apple");
        assert_eq!(strip_diacritics(""), "");
    }

    #[test]
    fn strip_diacritics_outros_idiomas() {
        assert_eq!(strip_diacritics("straße"), "strasse");
        assert_eq!(strip_diacritics("Müller"), "Muller");
        assert_eq!(strip_diacritics("smørrebrød"), "smorrebrod");
    }
}
