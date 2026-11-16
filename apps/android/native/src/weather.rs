//! Native UI and background-thread adapter for the shared weather engine.
use super::*;
pub(super) use puzzgen_weather::{Conditions, Mood, Preferences};
use puzzgen_weather::{Observation, Request, RequestGate, now};
pub(super) fn conditions(p: &Pocket) -> Conditions {
    let pref = &p.saved.weather;
    if pref.mode == Mood::Local
        && let Some(o) = &pref.cached
    {
        return Conditions {
            rain: o.rain,
            snow: o.snow,
            falling_snow: o.falling_snow,
            cloud: o.cloud,
            wind: o.wind,
            night: o.night,
            storm: o.storm,
        };
    }
    Conditions::for_mood(pref.mode)
}
#[derive(Resource)]
pub(super) struct Feed {
    tx: mpsc::Sender<(Request, Result<Observation, String>)>,
    rx: Mutex<mpsc::Receiver<(Request, Result<Observation, String>)>>,
    gate: RequestGate,
    pub status: String,
}
impl Default for Feed {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx: Mutex::new(rx),
            gate: RequestGate::default(),
            status: String::new(),
        }
    }
}
pub(super) fn update(
    mut p: ResMut<Pocket>,
    mut feed: ResMut<Feed>,
    windows: Query<&Window>,
    mut poll: Local<f32>,
    time: Res<Time>,
) {
    let replies: Vec<_> = feed
        .rx
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .try_iter()
        .collect();
    for (request, result) in replies {
        if let Some(result) = feed
            .gate
            .finish(&request, &mut p.saved.weather, result, now())
        {
            if platform::trace_input() {
                info!(
                    "WEATHER_FEED completed success={} cache_present={}",
                    result.is_ok(),
                    p.saved.weather.cached.is_some()
                );
            }
            feed.status = result.unwrap_or_else(|e| e);
            persist(&p);
            p.dirty = true;
        }
    }
    *poll += time.delta_secs();
    if *poll > 1. {
        *poll = 0.;
        if let Some(city) = platform::take_weather_city() {
            p.saved.weather.city = city;
            p.saved.weather.coordinates = None;
            p.saved.weather.cached = None;
            feed.gate.invalidate();
            feed.status.clear();
            p.saved.weather.mode = Mood::Local;
            p.dirty = true;
            persist(&p);
        }
        if let Some(coordinates) = platform::take_weather_position() {
            p.saved.weather.coordinates = Some(coordinates);
            p.saved.weather.cached = None;
            p.saved.weather.mode = Mood::Local;
            feed.gate.invalidate();
            feed.status = "Checking weather for your saved area…".into();
            p.dirty = true;
            persist(&p);
        }
        if let Some(status) = platform::take_weather_location_status() {
            feed.status = status;
            p.dirty = true;
        }
    }
    let garden_open = garden_walk::active(&p) || p.page == Page::Weather;
    let active = garden_open && windows.single().is_ok_and(|w| w.focused);
    if let Some(request) = feed.gate.start(&p.saved.weather, active, now()) {
        if platform::trace_input() {
            info!("WEATHER_FEED request_started");
        }
        let tx = feed.tx.clone();
        feed.status = "Checking the weather…".into();
        std::thread::spawn(move || {
            let result = if let Some(coordinates) = request.coordinates {
                puzzgen_weather::fetch_coordinates(&request.city, coordinates)
            } else {
                puzzgen_weather::fetch(&request.city)
            };
            let _ = tx.send((request, result));
        });
    }
}
pub(super) fn draw(
    c: &mut Commands,
    root: Entity,
    p: &mut Pocket,
    canvas: &Canvas,
    art: &home::HomeArt,
    feed: &Feed,
) {
    use home::{label, pill};
    let cream = Color::srgb(0.99, 0.95, 0.85);
    let ink = Color::srgb(0.055, 0.2, 0.14);
    label(
        c,
        root,
        "Garden weather",
        [24., 145., 432., 50.],
        32.,
        cream,
        &art.display,
        true,
    );
    label(
        c,
        root,
        "Choose a mood, a city, or your approximate location.",
        [24., 202., 432., 55.],
        21.,
        cream,
        &art.body,
        true,
    );
    for (i, m) in [
        Mood::Sunshine,
        Mood::Cloudy,
        Mood::Drizzle,
        Mood::Rain,
        Mood::Storm,
        Mood::Snow,
        Mood::Night,
        Mood::Local,
    ]
    .into_iter()
    .enumerate()
    {
        let selected = p.saved.weather.mode == m;
        pill(
            c,
            root,
            m.name(),
            [
                24. + (i % 2) as f32 * 220.,
                280. + (i / 2) as f32 * 76.,
                212.,
                62.,
            ],
            Action::WeatherMode(m),
            if selected { cream } else { ink },
            if selected { ink } else { cream },
            &art.body,
        );
    }
    let city = if p.saved.weather.city.is_empty() {
        "Choose city".to_owned()
    } else {
        format!("City: {}", p.saved.weather.city)
    };
    pill(
        c,
        root,
        &city,
        [24., 602., 432., 62.],
        Action::WeatherCity,
        if p.saved.weather.coordinates.is_none() {
            cream
        } else {
            ink
        },
        if p.saved.weather.coordinates.is_none() {
            ink
        } else {
            cream
        },
        &art.body,
    );
    pill(
        c,
        root,
        if p.saved.weather.coordinates.is_some() {
            "Use my location · selected"
        } else {
            "Use my location"
        },
        [24., 680., 432., 62.],
        Action::WeatherLocation,
        if p.saved.weather.coordinates.is_some() {
            cream
        } else {
            ink
        },
        if p.saved.weather.coordinates.is_some() {
            ink
        } else {
            cream
        },
        &art.body,
    );
    pill(
        c,
        root,
        if p.saved.weather.thunder {
            "Gentle thunder: ON"
        } else {
            "Gentle thunder: OFF"
        },
        [24., 758., 432., 62.],
        Action::Thunder,
        ink,
        cream,
        &art.body,
    );
    let info = p
        .saved
        .weather
        .cached
        .as_ref()
        .map(|o| {
            format!(
                "Saved forecast: {} ({} min old)\n",
                o.place,
                now().saturating_sub(o.at) / 60
            )
        })
        .unwrap_or_default();
    label(
        c,
        root,
        format!(
            "{info}{}\nDevice location is requested only when you tap Use my location. Your saved approximate area or entered city is sent to Open-Meteo for weather. No background tracking. Updates about every 20 minutes while the garden or weather page is open; offline, the saved forecast stays.\n\nWeather: Open-Meteo / GeoNames · CC BY 4.0\nopen-meteo.com · geonames.org\n\nSnow and puddles build and fade gradually. No flashing lightning.",
            feed.status
        ),
        [24., 848., 432., 390.],
        20.,
        cream,
        &art.body,
        false,
    );
    p.content_height = canvas.height.max(1280.);
}
