//! Western-chart display helpers: map the English wire values (`"Leo"`, `"Sun"`,
//! `"Fire"`, `"Fixed"`) to zh-TW labels and the CSS sign key used by the web
//! zodiac cards (`--sign-<key>` tokens). Pure, case-insensitive; never touches
//! the wire format (wire values stay capitalised English, see `ZODIAC`).

/// `(wire name, css key, zh-TW name)` — order matches `crate::ZODIAC`.
const SIGNS: [(&str, &str, &str); 12] = [
    ("Aries", "aries", "牡羊座"),
    ("Taurus", "taurus", "金牛座"),
    ("Gemini", "gemini", "雙子座"),
    ("Cancer", "cancer", "巨蟹座"),
    ("Leo", "leo", "獅子座"),
    ("Virgo", "virgo", "處女座"),
    ("Libra", "libra", "天秤座"),
    ("Scorpio", "scorpio", "天蠍座"),
    ("Sagittarius", "sagittarius", "射手座"),
    ("Capricorn", "capricorn", "摩羯座"),
    ("Aquarius", "aquarius", "水瓶座"),
    ("Pisces", "pisces", "雙魚座"),
];

const PLANETS: [(&str, &str); 10] = [
    ("Sun", "太陽"),
    ("Moon", "月亮"),
    ("Mercury", "水星"),
    ("Venus", "金星"),
    ("Mars", "火星"),
    ("Jupiter", "木星"),
    ("Saturn", "土星"),
    ("Uranus", "天王星"),
    ("Neptune", "海王星"),
    ("Pluto", "冥王星"),
];

const ELEMENTS: [(&str, &str); 4] = [
    ("Fire", "火象"),
    ("Earth", "土象"),
    ("Air", "風象"),
    ("Water", "水象"),
];

const QUALITIES: [(&str, &str); 3] = [("Cardinal", "本位"), ("Fixed", "固定"), ("Mutable", "變動")];

fn find_sign(name: &str) -> Option<&'static (&'static str, &'static str, &'static str)> {
    let n = name.trim();
    SIGNS
        .iter()
        .find(|(wire, _, _)| wire.eq_ignore_ascii_case(n))
}

fn lookup(table: &[(&str, &'static str)], key: &str) -> Option<&'static str> {
    let k = key.trim();
    table
        .iter()
        .find(|(en, _)| en.eq_ignore_ascii_case(k))
        .map(|(_, zh)| *zh)
}

/// zh-TW sign name (`"Leo"` → `"獅子座"`); unknown input is returned unchanged.
pub fn sign_zh(name: &str) -> String {
    find_sign(name)
        .map(|(_, _, zh)| (*zh).to_string())
        .unwrap_or_else(|| name.to_string())
}

/// Lower-case CSS key (`"Leo"` → `Some("leo")`), `None` for unknown signs.
pub fn sign_class(name: &str) -> Option<&'static str> {
    find_sign(name).map(|(_, key, _)| *key)
}

/// English `(element, quality)` of a sign (`"Leo"` → `("Fire", "Fixed")`), from
/// `crate::ZODIAC_META`; `None` for unknown signs. Lets the ascendant card show
/// the same traits the wire carries for `sunSign`/`moonSign`.
pub fn sign_traits(name: &str) -> Option<(&'static str, &'static str)> {
    let n = name.trim();
    crate::ZODIAC_META
        .iter()
        .find(|(wire, _, _, _)| wire.eq_ignore_ascii_case(n))
        .map(|(_, _, element, quality)| (*element, *quality))
}

/// zh-TW planet name (`"Sun"` → `"太陽"`); unknown input is returned unchanged.
pub fn planet_zh(name: &str) -> String {
    lookup(&PLANETS, name)
        .map(str::to_string)
        .unwrap_or_else(|| name.to_string())
}

/// zh-TW element (`"Fire"` → `"火象"`); unknown input is returned unchanged.
pub fn element_zh(element: &str) -> String {
    lookup(&ELEMENTS, element)
        .map(str::to_string)
        .unwrap_or_else(|| element.to_string())
}

/// zh-TW modality (`"Fixed"` → `"固定"`); unknown input is returned unchanged.
pub fn quality_zh(quality: &str) -> String {
    lookup(&QUALITIES, quality)
        .map(str::to_string)
        .unwrap_or_else(|| quality.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ZODIAC, ZODIAC_META};

    #[test]
    fn every_zodiac_wire_value_maps() {
        for sign in ZODIAC {
            let zh = sign_zh(sign);
            assert_ne!(zh, sign, "{sign} fell back");
            assert!(zh.ends_with('座'), "{sign} → {zh}");
            let class = sign_class(sign).unwrap_or_else(|| panic!("{sign} has no class"));
            assert_eq!(class, sign.to_ascii_lowercase());
        }
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(sign_zh("leo"), "獅子座");
        assert_eq!(sign_zh("LEO"), "獅子座");
        assert_eq!(sign_class("sAgItTaRiUs"), Some("sagittarius"));
        assert_eq!(planet_zh("sun"), "太陽");
        assert_eq!(planet_zh("MOON"), "月亮");
        assert_eq!(element_zh("fire"), "火象");
        assert_eq!(quality_zh("MUTABLE"), "變動");
    }

    #[test]
    fn unknown_values_pass_through() {
        assert_eq!(sign_zh("Ophiuchus"), "Ophiuchus");
        assert_eq!(sign_zh(""), "");
        assert_eq!(sign_class("Ophiuchus"), None);
        assert_eq!(sign_class(""), None);
        assert_eq!(planet_zh("Chiron"), "Chiron");
        assert_eq!(element_zh("Aether"), "Aether");
        assert_eq!(quality_zh(""), "");
        assert_eq!(sign_traits("Ophiuchus"), None);
    }

    #[test]
    fn sign_traits_case_insensitive() {
        assert_eq!(sign_traits("leo"), Some(("Fire", "Fixed")));
        assert_eq!(sign_traits("PISCES"), Some(("Water", "Mutable")));
        for sign in ZODIAC {
            assert!(sign_traits(sign).is_some(), "{sign}");
        }
    }

    #[test]
    fn common_planets_have_zh() {
        for p in [
            "Sun", "Moon", "Mercury", "Venus", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune",
            "Pluto",
        ] {
            assert_ne!(planet_zh(p), p, "{p} fell back");
        }
    }

    #[test]
    fn sign_meta_elements_and_qualities_map() {
        for (name, _, element, quality) in ZODIAC_META {
            assert_ne!(element_zh(element), element, "{name} element {element}");
            assert_ne!(quality_zh(quality), quality, "{name} quality {quality}");
        }
    }
}
