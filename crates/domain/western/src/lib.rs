//! Western astrology — solar-ephemeris for all bodies (Sun/Moon via dedicated
//! functions, planets via the patched `Body::elements()` + `planet_apparent_ecliptic`
//! geocentric apparent ecliptic). vsop87 is NOT used: its `*.longitude()` is
//! heliocentric J2000, not geocentric apparent — that was the Phase A bug caught
//! by the §8.2 event table.

use ft_schema::WesternChartV3;
use solar_ephemeris::{elpmpp02, planets, time, timescales::AstroTime, Body};

pub fn calculate(jd_utc: f64, lat_deg: f64, lon_east_deg: f64) -> WesternChartV3 {
    let astro = AstroTime::from_jd_utc(jd_utc);
    let jd_tt = astro.jd_tt;
    let t = time::centuries(jd_tt);
    let (dpsi, deps) = time::nutation_deg(t);
    let eps_true = time::mean_obliquity_deg(t) + deps;

    // Sun / Moon — geocentric apparent ecliptic, validated against JPL HORIZONS.
    let sun_lon = planets::sun_apparent_ecliptic(jd_tt, dpsi)
        .0
        .rem_euclid(360.0);
    let moon_lon = elpmpp02::moon_apparent_ecliptic(jd_tt, dpsi)
        .0
        .rem_euclid(360.0);

    let gast = time::gast_deg(astro.jd_ut1, dpsi, eps_true);
    let lst = (gast + lon_east_deg).rem_euclid(360.0);
    let asc_lon = ascendant_lon(lst, lat_deg, eps_true);

    // Planets — geocentric apparent ecliptic (light-time + aberration + Meeus-21
    // precession + nutation). `Body::elements()` is the patched-pub accessor; each
    // planet's `&Planet` feeds planet_apparent_ecliptic.
    let mut planets: Vec<(&str, f64)> = vec![("Sun", sun_lon), ("Moon", moon_lon)];
    for body in [
        Body::Mercury,
        Body::Venus,
        Body::Mars,
        Body::Jupiter,
        Body::Saturn,
        Body::Uranus,
        Body::Neptune,
    ] {
        if let Some(el) = body.elements() {
            let lon = planets::planet_apparent_ecliptic(el, jd_tt, dpsi)
                .0
                .rem_euclid(360.0);
            planets.push((body_name(body), lon));
        }
    }

    WesternChartV3::from_longitudes(planets, asc_lon, jd_utc)
}

/// The planet display names (mirror the `Body::name()` the upstream crate keeps private).
fn body_name(b: Body) -> &'static str {
    match b {
        Body::Mercury => "Mercury",
        Body::Venus => "Venus",
        Body::Mars => "Mars",
        Body::Jupiter => "Jupiter",
        Body::Saturn => "Saturn",
        Body::Uranus => "Uranus",
        Body::Neptune => "Neptune",
        _ => "?",
    }
}

fn ascendant_lon(lst_deg: f64, lat_deg: f64, eps_deg: f64) -> f64 {
    // Ascendant = ecliptic longitude of the eastern rising point on the horizon.
    // Meeus / Duffett-Smith: tan λ = −cos θ / (sin θ cos ε + tan φ sin ε), θ = local
    // apparent sidereal time. The well-formed atan2 form
    //     λ_ASC = atan2(cos θ, −(sin θ cos ε + tan φ sin ε))
    // is ALREADY the eastern rising point for every |φ| < 90°−ε (no branch selection).
    // The opposite curve (λ+180) is the descendant — do not add +180 / azimuth filtering
    // (a previous attempt did, and mis-picked the descendant for many epochs).
    // Source for φ<0: the tan φ term just changes sign, same atan2.
    let theta = lst_deg.to_radians();
    let phi = lat_deg.to_radians();
    let eps = eps_deg.to_radians();
    let den = theta.sin() * eps.cos() + phi.tan() * eps.sin();
    theta.cos().atan2(-den).to_degrees().rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sun_in_taurus_mid_may() {
        let jd = 2448028.5;
        let c = calculate(jd, 25.0, 121.5);
        let sun = c.planets.iter().find(|p| p.name == "Sun").unwrap();
        assert!(
            sun.longitude >= 30.0 && sun.longitude < 60.0,
            "sun lon {} not Taurus, sign {} deg {}",
            sun.longitude,
            sun.sign,
            sun.degree
        );
    }
    #[test]
    fn moon_longitude_reasonable() {
        let c = calculate(2448028.5, 25.0, 121.5);
        let moon = c.planets.iter().find(|p| p.name == "Moon").unwrap();
        assert!(moon.longitude >= 0.0 && moon.longitude < 360.0);
    }
    #[test]
    fn all_planets_present() {
        let c = calculate(2448028.5, 25.0, 121.5);
        assert_eq!(c.planets.len(), 9);
        assert!(c.ascendant.longitude >= 0.0 && c.ascendant.longitude < 360.0);
    }
}
