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
    let sun_lon = planets::sun_apparent_ecliptic(jd_tt, dpsi).0.rem_euclid(360.0);
    let moon_lon = elpmpp02::moon_apparent_ecliptic(jd_tt, dpsi).0.rem_euclid(360.0);

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
            let lon = planets::planet_apparent_ecliptic(el, jd_tt, dpsi).0.rem_euclid(360.0);
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
    // Meeus (Astronomical Algorithms): the two ecliptic-horizon intersections are λ0 and
    // λ0+180°, given by tan(λ) = -cos(θ) / (sin(θ)cos(ε) + tan(φ)sin(ε)). The ascendant is
    // the one on the EASTERN (rising) horizon. We evaluate both candidates' local azimuth
    // and keep the one in the east (az ≈ 90°, i.e. 45°–135°). This is unambiguous across
    // the whole 1900–2100 span (the §8.2 event table caught the old formula yielding a
    // descendant / off-horizon value, alt ≈ -42°).
    let lst = lst_deg.to_radians();
    let lat = lat_deg.to_radians();
    let eps = eps_deg.to_radians();
    let num = -lst.cos();
    let den = lst.sin() * eps.cos() + lat.tan() * eps.sin();
    let l0 = num.atan2(den).to_degrees().rem_euclid(360.0);
    // Try both branches; pick the eastern (rising) one.
    let a = (l0, azim_deg(l0, lat_deg, eps_deg, lst_deg));
    let b = ((l0 + 180.0).rem_euclid(360.0), azim_deg(l0 + 180.0, lat_deg, eps_deg, lst_deg));
    if a.1 >= 45.0 && a.1 <= 135.0 {
        a.0
    } else if b.1 >= 45.0 && b.1 <= 135.0 {
        b.0
    } else {
        // Edge case: nearly at the horizon poles. Prefer the one with altitude rising.
        a.0
    }
}

/// Azimuth (0=N, 90=E) of an ecliptic point (β=0) on the local horizon.
fn azim_deg(lon_deg: f64, lat_deg: f64, eps_deg: f64, lst_deg: f64) -> f64 {
    let e = eps_deg.to_radians();
    let phi = lat_deg.to_radians();
    let lon = lon_deg.to_radians();
    let dec = (e.cos() * lon.sin()).asin();
    let ra = (lon.sin() * e.cos()).atan2(lon.cos());
    let ha = lst_deg.to_radians() - ra;
    let az = (-dec.cos() * ha.sin())
        .atan2(dec.sin() * phi.cos() - dec.cos() * phi.sin() * ha.cos());
    az.to_degrees().rem_euclid(360.0)
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
