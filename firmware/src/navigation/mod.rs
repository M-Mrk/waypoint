use defmt::warn;
use heapless::String;
use libm::{asin, atan2, cos, pow, sin, sqrt};

use crate::inputs::gnss::GnssData;

pub struct Coordinate {
    pub lat: f64,
    pub long: f64,
    pub alt: Option<f32>,
}
impl Coordinate {
    pub fn from_gnss(fix: GnssData) -> Result<Self, ()> {
        if fix.latitude.is_none() || fix.longitude.is_none() {
            return Err(());
        }

        Ok(Self {
            lat: fix.latitude.unwrap(),
            long: fix.longitude.unwrap(),
            alt: fix.altitude_m,
        })
    }
}

pub struct Waypoint {
    pub name: String<15>,
    pub coordinate: Coordinate,
}

pub struct WaypointDifference {
    distance_m: f64,
    height_delta: Option<i32>,
    needed_bearing: f64,
}

fn haversine_distance(from: &Coordinate, to: &Coordinate) -> f64 {
    let lat1 = from.lat.to_radians();
    let lat2 = to.lat.to_radians();
    let dlat = (to.lat - from.lat).to_radians();
    let dlon = (to.long - from.long).to_radians();

    let a = pow(sin(dlat / 2.0), 2_f64) + cos(lat1) * cos(lat2) * pow(sin(dlon / 2.0), 2_f64);

    let c = 2.0 * asin(sqrt(a));
    6_371_000.0 * c
}

fn bearing(from: &Coordinate, to: &Coordinate) -> f64 {
    let lat1 = from.lat.to_radians();
    let lat2 = to.lat.to_radians();
    let dlon = (to.long - from.long).to_radians();

    let y = sin(dlon) * cos(lat2);
    let x = cos(lat1) * sin(lat2) - sin(lat1) * cos(lat2) * cos(dlon);

    let bearing = atan2(y, x).to_degrees();
    (bearing + 360.0) % 360.0
}

fn height_difference(from: &Coordinate, to: &Coordinate) -> Option<i32> {
    if from.alt.is_none() || to.alt.is_none() {
        warn!("Can't calculate height difference, as missing height");
        return None;
    }
    return Some((from.alt.unwrap() - to.alt.unwrap()) as i32);
}

pub async fn calculate_waypoint_data(
    waypoint: Waypoint,
    last_fix: GnssData,
) -> Result<WaypointDifference, ()> {
    let current_position = Coordinate::from_gnss(last_fix)?;

    Ok(WaypointDifference {
        distance_m: haversine_distance(&current_position, &waypoint.coordinate),
        needed_bearing: bearing(&current_position, &waypoint.coordinate),
        height_delta: height_difference(&current_position, &waypoint.coordinate),
    })
}
