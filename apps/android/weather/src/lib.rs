//! Shared weather client/cache policy, independent of Bevy and Android.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[path = "../../native/src/weather_rules.rs"]
mod weather_rules;
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize)]
pub enum Mood {
    #[default]
    Sunshine,
    Cloudy,
    Drizzle,
    Rain,
    Storm,
    Snow,
    Night,
    Local,
}
impl Mood {
    pub fn name(self) -> &'static str {
        match self {
            Self::Sunshine => "Sunshine",
            Self::Cloudy => "Cloudy",
            Self::Drizzle => "Drizzle",
            Self::Rain => "Rain",
            Self::Storm => "Storm",
            Self::Snow => "Snow",
            Self::Night => "Moonlight",
            Self::Local => "Local weather",
        }
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Preferences {
    pub mode: Mood,
    pub city: String,
    pub cached: Option<Observation>,
    #[serde(default)]
    pub coordinates: Option<[f64; 2]>,
    #[serde(default)]
    pub thunder: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Observation {
    #[serde(default)]
    pub coordinates: Option<[f64; 2]>,
    pub city: String,
    pub place: String,
    pub at: u64,
    pub rain: f32,
    pub snow: f32,
    #[serde(default)]
    pub falling_snow: bool,
    pub cloud: f32,
    pub wind: f32,
    pub night: bool,
    pub storm: bool,
}
#[derive(Clone, Copy)]
pub struct Conditions {
    pub rain: f32,
    pub snow: f32,
    pub falling_snow: bool,
    pub cloud: f32,
    pub wind: f32,
    pub night: bool,
    pub storm: bool,
}
impl Conditions {
    pub fn for_mood(m: Mood) -> Self {
        Self {
            rain: match m {
                Mood::Drizzle => 0.22,
                Mood::Rain => 0.7,
                Mood::Storm => 1.,
                _ => 0.,
            },
            snow: if m == Mood::Snow { 1. } else { 0. },
            falling_snow: m == Mood::Snow,
            cloud: match m {
                Mood::Cloudy | Mood::Rain | Mood::Storm | Mood::Snow => 1.,
                Mood::Drizzle => 0.7,
                _ => 0.1,
            },
            wind: if m == Mood::Storm { 1. } else { 0.25 },
            night: m == Mood::Night,
            storm: m == Mood::Storm,
        }
    }
}
pub fn fetch(city: &str) -> Result<Observation, String> {
    let offline = || "Weather unavailable; keeping the last mood. Try again later.".to_owned();
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| offline())?;
    fetch_from(
        &client,
        city,
        "https://geocoding-api.open-meteo.com/v1/search",
        "https://api.open-meteo.com/v1/forecast",
        now(),
    )
}
fn fetch_from(
    client: &reqwest::blocking::Client,
    city: &str,
    geo_url: &str,
    forecast_url: &str,
    at: u64,
) -> Result<Observation, String> {
    let offline = || {
        "Weather unavailable; using saved conditions when available. Try again later.".to_owned()
    };
    // Separate client and public endpoints: the private puzzle key never leaves its API origin.
    let place = resolve_city(client, city, geo_url)?;
    let lat = place["latitude"]
        .as_f64()
        .filter(|v| v.abs() <= 90.)
        .ok_or_else(offline)?;
    let lon = place["longitude"]
        .as_f64()
        .filter(|v| v.abs() <= 180.)
        .ok_or_else(offline)?;
    let name = ["name", "admin1", "country_code"]
        .iter()
        .filter_map(|k| place[k].as_str())
        .collect::<Vec<_>>()
        .join(", ");
    fetch_forecast(client, city, name, lat, lon, forecast_url, at)
}
fn fetch_forecast(
    client: &reqwest::blocking::Client,
    city: &str,
    name: String,
    lat: f64,
    lon: f64,
    forecast_url: &str,
    at: u64,
) -> Result<Observation, String> {
    let offline = || {
        "Weather unavailable; using saved conditions when available. Try again later.".to_owned()
    };
    let data: Value=client.get(forecast_url)
        .query(&[("latitude",lat.to_string()),("longitude",lon.to_string()),
            ("current","temperature_2m,is_day,rain,showers,snowfall,weather_code,cloud_cover,wind_speed_10m".into()),
            ("hourly","snow_depth".into()),("past_hours","24".into()),("forecast_hours","1".into()),("timezone","auto".into())])
        .send().and_then(|r|r.error_for_status()).map_err(|_|offline())?.json().map_err(|_|offline())?;
    parse(&data, city, name, at)
}
pub fn rounded_coordinates(latitude: f64, longitude: f64) -> Option<[f64; 2]> {
    if !latitude.is_finite()
        || !longitude.is_finite()
        || latitude.abs() > 90.
        || longitude.abs() > 180.
    {
        return None;
    }
    Some([
        (latitude * 100.).round() / 100.,
        (longitude * 100.).round() / 100.,
    ])
}
pub fn fetch_coordinates(city: &str, coordinates: [f64; 2]) -> Result<Observation, String> {
    let coordinates = rounded_coordinates(coordinates[0], coordinates[1])
        .ok_or("Location unavailable. Choose a city instead.")?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Weather unavailable.".to_owned())?;
    fetch_coordinates_from(
        &client,
        city,
        coordinates,
        "https://api.open-meteo.com/v1/forecast",
        now(),
    )
}
fn fetch_coordinates_from(
    client: &reqwest::blocking::Client,
    city: &str,
    coordinates: [f64; 2],
    url: &str,
    at: u64,
) -> Result<Observation, String> {
    let mut observation = fetch_forecast(
        client,
        city,
        "My approximate location".into(),
        coordinates[0],
        coordinates[1],
        url,
        at,
    )?;
    observation.coordinates = Some(coordinates);
    Ok(observation)
}
fn resolve_city(
    client: &reqwest::blocking::Client,
    city: &str,
    geo_url: &str,
) -> Result<Value, String> {
    let city = city.trim();
    if city.is_empty() || city.chars().count() > 100 {
        return Err("Enter a city and optional state or country.".into());
    }
    let mut attempts = Vec::new();
    if let Some((name, region)) = city.split_once(',') {
        attempts.push((name.trim().to_owned(), Some(region.trim().to_owned())));
    } else {
        attempts.push((city.to_owned(), None));
        // Open-Meteo searches place names, not free-form city/state addresses.
        // Try at most two bounded suffix splits, validating the region before selection.
        let words: Vec<_> = city.split_whitespace().collect();
        for suffix in 1..=2 {
            if words.len() > suffix {
                let split = words.len() - suffix;
                attempts.push((words[..split].join(" "), Some(words[split..].join(" "))));
            }
        }
    }
    fn normalized(value: &str) -> String {
        value
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect()
    }
    for (name, region) in attempts {
        let geo: Value = client
            .get(geo_url)
            .query(&[("name", name.as_str()), ("count", "20"), ("language", "en")])
            .send()
            .and_then(|r| r.error_for_status())
            .map_err(|_| "Weather lookup unavailable; keeping saved conditions.".to_owned())?
            .json()
            .map_err(|_| "Weather lookup returned an invalid response.".to_owned())?;
        if let Some(places) = geo["results"].as_array() {
            for place in places {
                let matches = region.as_ref().is_none_or(|region| {
                    region.split(',').all(|part| {
                        let qualifier = normalized(part);
                        !qualifier.is_empty()
                            && ["admin1", "admin2", "country", "country_code"]
                                .iter()
                                .any(|key| {
                                    place[key]
                                        .as_str()
                                        .is_some_and(|value| normalized(value) == qualifier)
                                })
                    })
                });
                if matches {
                    return Ok(place.clone());
                }
            }
        }
    }
    Err("City or region not found. Try City, full state name or country.".into())
}
fn parse(data: &Value, city: &str, place: String, at: u64) -> Result<Observation, String> {
    let c = &data["current"];
    let code = c["weather_code"]
        .as_u64()
        .ok_or("Weather response was incomplete; keeping the last mood.")?;
    let incomplete = "Weather response was incomplete; keeping the last mood.";
    let number = |name: &str| {
        c[name]
            .as_f64()
            .filter(|n| n.is_finite() && *n >= 0.)
            .map(|n| n as f32)
            .ok_or(incomplete)
    };
    let is_day = c["is_day"].as_u64().filter(|n| *n <= 1).ok_or(incomplete)?;
    let snowfall = number("snowfall")?;
    let current_time = c["time"].as_str().unwrap_or("");
    let snow_index = data["hourly"]["time"].as_array().and_then(|times| {
        times
            .iter()
            .rposition(|t| t.as_str().is_some_and(|t| t <= current_time))
    });
    let snow_depth = snow_index
        .and_then(|i| data["hourly"]["snow_depth"][i].as_f64())
        .unwrap_or(0.) as f32;
    let (rain, snow, falling_snow) = weather_rules::precipitation(
        code,
        number("rain")? + number("showers")?,
        snowfall,
        snow_depth,
    );
    Ok(Observation {
        coordinates: None,
        city: city.into(),
        place,
        at,
        rain,
        snow,
        falling_snow,
        cloud: (number("cloud_cover")? / 100.).clamp(0., 1.),
        wind: (number("wind_speed_10m")? / 40.).clamp(0., 1.),
        night: is_day == 0,
        storm: matches!(code, 95..=99),
    })
}

pub const REFRESH_SECONDS: u64 = 20 * 60;
#[derive(Clone, Debug)]
pub struct Request {
    pub city: String,
    pub coordinates: Option<[f64; 2]>,
    generation: u64,
}
#[derive(Default)]
pub struct RequestGate {
    pending: Option<Request>,
    generation: u64,
    retry_after: u64,
}
impl RequestGate {
    pub fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.retry_after = 0;
        // Keep the old request pending until it returns: at most one worker.
    }
    pub fn start(&mut self, preferences: &Preferences, active: bool, at: u64) -> Option<Request> {
        if !active
            || preferences.mode != Mood::Local
            || (preferences.city.trim().is_empty() && preferences.coordinates.is_none())
            || self.pending.is_some()
            || at < self.retry_after
        {
            return None;
        }
        if preferences.cached.as_ref().is_some_and(|o| {
            o.city == preferences.city
                && o.coordinates == preferences.coordinates
                && at >= o.at
                && at - o.at < REFRESH_SECONDS
        }) {
            return None;
        }
        let request = Request {
            city: preferences.city.clone(),
            coordinates: preferences.coordinates,
            generation: self.generation,
        };
        self.pending = Some(request.clone());
        Some(request)
    }
    pub fn finish(
        &mut self,
        request: &Request,
        preferences: &mut Preferences,
        result: Result<Observation, String>,
        at: u64,
    ) -> Option<Result<String, String>> {
        let pending = self.pending.as_ref()?;
        if pending.generation != request.generation
            || pending.city != request.city
            || pending.coordinates != request.coordinates
        {
            return None;
        }
        self.pending = None;
        if request.generation != self.generation
            || request.city != preferences.city
            || request.coordinates != preferences.coordinates
        {
            return None;
        }
        match result {
            Ok(observation)
                if observation.city == preferences.city
                    && observation.coordinates == preferences.coordinates =>
            {
                let message = format!("Weather for {}", observation.place);
                preferences.cached = Some(observation);
                self.retry_after = 0;
                Some(Ok(message))
            }
            _ => {
                self.retry_after = at.saturating_add(REFRESH_SECONDS);
                Some(Err(
                    "Weather unavailable; using saved conditions when available. Try again later."
                        .into(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn device_coordinates_are_validated_and_rounded() {
        assert_eq!(
            rounded_coordinates(39.76123, -84.19234),
            Some([39.76, -84.19])
        );
        for (lat, lon) in [(f64::NAN, 0.), (91., 0.), (0., 181.), (0., f64::INFINITY)] {
            assert!(rounded_coordinates(lat, lon).is_none());
        }
    }
    #[test]
    fn location_weather_bypasses_city_lookup() {
        let (url, requests, task) = server(vec![(200, forecast())]);
        let client = reqwest::blocking::Client::new();
        let observation =
            fetch_coordinates_from(&client, "Saved manual city", [39.76, -84.19], &url, 100)
                .unwrap();
        task.join().unwrap();
        assert_eq!(observation.coordinates, Some([39.76, -84.19]));
        assert_eq!(observation.city, "Saved manual city");
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].contains("latitude=39.76") && requests[0].contains("longitude=-84.19"));
        assert!(!requests[0].contains("name="));
    }
    #[test]
    fn changing_location_or_returning_to_city_rejects_stale_forecasts() {
        let mut p = preferences();
        p.coordinates = Some([39., -84.]);
        let mut gate = RequestGate::default();
        let old = gate.start(&p, true, 100).unwrap();
        p.coordinates = Some([40., -85.]);
        gate.invalidate();
        let mut old_observation = observation(100);
        old_observation.coordinates = old.coordinates;
        assert!(
            gate.finish(&old, &mut p, Ok(old_observation), 101)
                .is_none()
        );
        let current = gate.start(&p, true, 102).unwrap();
        let mut latest = observation(102);
        latest.coordinates = current.coordinates;
        assert!(
            gate.finish(&current, &mut p, Ok(latest), 103)
                .unwrap()
                .is_ok()
        );
        assert_eq!(p.city, "Dayton");
        p.coordinates = None;
        gate.invalidate();
        assert!(gate.start(&p, true, 104).is_some());
    }
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{Arc, Mutex},
        thread,
    };
    fn forecast() -> Value {
        json!({"current":{"time":"2026-11-15T12:15","weather_code":61,"rain":1.,"showers":0.,"snowfall":0.,"cloud_cover":90.,"wind_speed_10m":12.,"is_day":1},"hourly":{"time":["2026-11-15T12:00","2026-11-15T13:00"],"snow_depth":[0.1,1.]}})
    }
    fn observation(at: u64) -> Observation {
        parse(&forecast(), "Dayton", "Dayton, Ohio".into(), at).unwrap()
    }
    fn preferences() -> Preferences {
        Preferences {
            mode: Mood::Local,
            city: "Dayton".into(),
            ..Default::default()
        }
    }
    fn server(
        responses: Vec<(u16, Value)>,
    ) -> (String, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>) {
        let socket = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", socket.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(vec![]));
        let log = requests.clone();
        let task = thread::spawn(move || {
            for (status, body) in responses {
                let (mut stream, _) = socket.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut data = vec![];
                let mut buf = [0; 1024];
                while !data.windows(4).any(|v| v == b"\r\n\r\n") {
                    let n = stream.read(&mut buf).unwrap();
                    if n == 0 {
                        break;
                    }
                    data.extend_from_slice(&buf[..n]);
                }
                log.lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&data).into_owned());
                let body = body.to_string();
                write!(stream,"HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
        });
        (url, requests, task)
    }
    #[test]
    fn live_shaped_http_fetch_parses_current_hour_and_has_no_credentials() {
        let (url, requests, task) = server(vec![
            (
                200,
                json!({"results":[{"latitude":39.76,"longitude":-84.19,"name":"Dayton","admin1":"Ohio","country_code":"US"}]}),
            ),
            (200, forecast()),
        ]);
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let o = fetch_from(
            &client,
            "Dayton",
            &format!("{url}/geo"),
            &format!("{url}/forecast"),
            100,
        )
        .unwrap();
        task.join().unwrap();
        assert_eq!(o.rain, 0.5);
        assert_eq!(o.snow, 0.8);
        assert!(!o.falling_snow);
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].contains("name=Dayton"));
        assert!(requests[1].contains("latitude=39.76"));
        assert!(
            requests
                .iter()
                .all(|r| !r.to_lowercase().contains("authorization:"))
        );
    }
    #[test]
    fn city_and_state_is_resolved_without_silently_selecting_another_region() {
        let (url, requests, task) = server(vec![
            (200, json!({"results":[]})),
            (
                200,
                json!({"results":[
                {"latitude":42.1,"longitude":-72.6,"name":"Springfield","admin1":"Massachusetts","country_code":"US"},
                {"latitude":39.8,"longitude":-89.6,"name":"Springfield","admin1":"Illinois","country_code":"US"}]}),
            ),
            (200, forecast()),
        ]);
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let o = fetch_from(
            &client,
            "Springfield Illinois",
            &format!("{url}/geo"),
            &format!("{url}/forecast"),
            100,
        )
        .unwrap();
        task.join().unwrap();
        assert!(o.place.contains("Illinois"));
        let requests = requests.lock().unwrap();
        assert!(requests[1].contains("name=Springfield&"));
        assert!(requests[2].contains("latitude=39.8"));
    }
    #[test]
    fn unmatched_region_is_rejected_instead_of_using_first_city() {
        let (url, _, task) = server(vec![(
            200,
            json!({"results":[{"name":"Springfield","admin1":"Massachusetts","country_code":"US"}]}),
        )]);
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        assert!(resolve_city(&client, "Springfield, Illinois", &url).is_err());
        task.join().unwrap();
    }
    #[test]
    fn fresh_cache_survives_serialization_and_prevents_network_after_restart() {
        let mut p = preferences();
        p.cached = Some(observation(100));
        let restored: Preferences =
            serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        let mut gate = RequestGate::default();
        assert!(gate.start(&restored, true, 1299).is_none());
        assert!(gate.start(&restored, true, 1300).is_some());
    }
    #[test]
    fn offline_failure_keeps_cached_conditions_and_backs_off() {
        let mut p = preferences();
        p.cached = Some(observation(100));
        let before = serde_json::to_value(&p.cached).unwrap();
        let mut gate = RequestGate::default();
        let request = gate.start(&p, true, 1300).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .timeout(Duration::from_millis(200))
            .build()
            .unwrap();
        let failure = fetch_from(&client, "Dayton", &url, &url, 1300);
        assert!(failure.is_err());
        assert!(
            gate.finish(&request, &mut p, failure, 1300)
                .unwrap()
                .is_err()
        );
        assert_eq!(serde_json::to_value(&p.cached).unwrap(), before);
        assert!(gate.start(&p, true, 2499).is_none());
        assert!(gate.start(&p, true, 2500).is_some());
    }
    #[test]
    fn background_pending_and_manual_moods_never_start_requests() {
        let mut p = preferences();
        let mut gate = RequestGate::default();
        assert!(gate.start(&p, false, 100).is_none());
        p.mode = Mood::Rain;
        assert!(gate.start(&p, true, 100).is_none());
        p.mode = Mood::Local;
        assert!(gate.start(&p, true, 100).is_some());
        assert!(gate.start(&p, true, 101).is_none());
    }
    #[test]
    fn city_changes_reject_even_an_a_to_b_to_a_stale_reply() {
        let mut p = preferences();
        let mut gate = RequestGate::default();
        let old = gate.start(&p, true, 100).unwrap();
        p.city = "Paris".into();
        gate.invalidate();
        p.city = "Dayton".into();
        gate.invalidate();
        assert!(gate.start(&p, true, 101).is_none());
        assert!(
            gate.finish(&old, &mut p, Ok(observation(100)), 102)
                .is_none()
        );
        assert!(p.cached.is_none());
        let new = gate.start(&p, true, 103).unwrap();
        assert!(
            gate.finish(&new, &mut p, Ok(observation(103)), 103)
                .unwrap()
                .is_ok()
        );
    }
    #[test]
    fn malformed_forecast_and_server_errors_are_failures() {
        assert!(parse(&json!({}), "Dayton", "".into(), 100).is_err());
        let mut missing = forecast();
        missing["current"].as_object_mut().unwrap().remove("rain");
        assert!(parse(&missing, "Dayton", "".into(), 100).is_err());
        let (url, _, task) = server(vec![(503, json!({"error":true}))]);
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        assert!(fetch_from(&client, "Dayton", &url, &url, 100).is_err());
        task.join().unwrap();
    }
    #[test]
    fn clock_rollback_and_wrong_city_do_not_make_cache_permanently_fresh() {
        let mut p = preferences();
        p.cached = Some(observation(100));
        assert!(RequestGate::default().start(&p, true, 99).is_some());
        p.city = "Paris".into();
        assert!(RequestGate::default().start(&p, true, 101).is_some());
    }
    #[test]
    #[ignore = "explicit live Open-Meteo request"]
    fn live_open_meteo_fetch() {
        let o = fetch("Dayton").expect("live geocoding and forecast must succeed");
        assert!(o.place.contains("Dayton"));
        assert!((0.0..=1.0).contains(&o.rain));
        println!(
            "Live Open-Meteo success: rain={}, snow={}, cloud={}, night={}",
            o.rain, o.snow, o.cloud, o.night
        );
    }
}
