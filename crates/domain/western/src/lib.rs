//! Western astrology — hybrid: VSOP87 (planets) + ELP-MPP02 (Moon).

use ft_schema::WesternChartV3;
use solar_ephemeris::{elpmpp02, planets, time, timescales::AstroTime};

pub fn calculate(jd_utc: f64, lat_deg: f64, lon_east_deg: f64) -> WesternChartV3 {
    let astro = AstroTime::from_jd_utc(jd_utc);
    let jd_tt = astro.jd_tt;
    let t = time::centuries(jd_tt);
    let (dpsi, deps) = time::nutation_deg(t);
    let eps_true = time::mean_obliquity_deg(t) + deps;

    // Sun via solar-ephemeris (public, correct)
    let sun_lon = {
        let (lon, _, _) = planets::sun_apparent_ecliptic(jd_tt, dpsi);
        lon.rem_euclid(360.0)
    };

    let moon_lon = {
        let (lon, _lat, _dist) = elpmpp02::moon_apparent_ecliptic(jd_tt, dpsi);
        lon.rem_euclid(360.0)
    };

    let gast = time::gast_deg(astro.jd_ut1, dpsi, eps_true);
    let lst = (gast + lon_east_deg).rem_euclid(360.0);
    let asc_lon = ascendant_lon(lst, lat_deg, eps_true);

    // Planets via vsop87 — longitude() returns RADIANS, convert to degrees
    let to_deg = |rad: f64| (rad * 180.0 / std::f64::consts::PI).rem_euclid(360.0);
    let planets = vec![
        ("Sun", sun_lon),
        ("Moon", moon_lon),
        ("Mercury", to_deg(vsop87::vsop87d::mercury(jd_tt).longitude())),
        ("Venus", to_deg(vsop87::vsop87d::venus(jd_tt).longitude())),
        ("Mars", to_deg(vsop87::vsop87d::mars(jd_tt).longitude())),
        ("Jupiter", to_deg(vsop87::vsop87d::jupiter(jd_tt).longitude())),
        ("Saturn", to_deg(vsop87::vsop87d::saturn(jd_tt).longitude())),
        ("Uranus", to_deg(vsop87::vsop87d::uranus(jd_tt).longitude())),
        ("Neptune", to_deg(vsop87::vsop87d::neptune(jd_tt).longitude())),
    ];

    WesternChartV3::from_longitudes(planets, asc_lon, jd_utc)
}

fn ascendant_lon(lst_deg: f64, lat_deg: f64, eps_deg: f64) -> f64 {
    let lst = lst_deg.to_radians();
    let lat = lat_deg.to_radians();
    let eps = eps_deg.to_radians();
    let num = lst.cos();
    let den = -lst.sin() * eps.sin() - lat.tan() * eps.cos();
    num.atan2(den).to_degrees().rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sun_in_taurus_mid_may() {
        let jd = 2448028.5;
        let c = calculate(jd, 25.0, 121.5);
        let sun = c.planets.iter().find(|p| p.name == "Sun").unwrap();
        assert!(sun.longitude >= 30.0 && sun.longitude < 60.0, "sun lon {} not Taurus, sign {} deg {}", sun.longitude, sun.sign, sun.degree);
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
