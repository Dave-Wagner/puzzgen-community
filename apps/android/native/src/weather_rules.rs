//! Weather-code interpretation kept independent of rendering and network I/O.
pub fn precipitation(
    code: u64,
    rain_mm: f32,
    snowfall_cm: f32,
    snow_depth_m: f32,
) -> (f32, f32, bool) {
    let falling = snowfall_cm > 0. || matches!(code, 71..=77 | 85 | 86);
    let snow = if falling {
        1.
    } else {
        (snow_depth_m * 8.).clamp(0., 1.)
    };
    let rain = if falling {
        0.
    } else if rain_mm > 0. {
        (rain_mm / 2.).clamp(0.12, 1.)
    } else {
        match code {
            51..=57 => 0.2,
            61..=67 | 80..=82 => 0.65,
            95..=99 => 1.,
            _ => 0.,
        }
    };
    (rain, snow, falling)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snow_on_ground_does_not_create_a_snowstorm_or_suppress_rain() {
        assert_eq!(precipitation(0, 0., 0., 0.2), (0., 1., false));
        assert_eq!(precipitation(61, 1., 0., 0.2), (0.5, 1., false));
        assert_eq!(precipitation(73, 0., 0.5, 0.), (0., 1., true));
    }
    #[test]
    fn drizzle_and_downpour_have_different_density() {
        assert!(precipitation(51, 0., 0., 0.).0 < precipitation(65, 5., 0., 0.).0);
        assert_eq!(precipitation(95, 0., 0., 0.).0, 1.);
        assert_eq!(precipitation(3, 0., 0., 0.), (0., 0., false));
    }
}
