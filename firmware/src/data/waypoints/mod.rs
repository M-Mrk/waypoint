use defmt::{Format};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use heapless::{String, Vec};

use crate::inputs::gnss::GnssData;

#[derive(Format, Clone, Copy)]
pub struct Coordinate {
    pub lat: f64,
    pub long: f64,
    pub alt: Option<f32>,
}
impl Coordinate {
    pub fn from_gnss(fix: &GnssData) -> Result<Self, ()> {
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

pub const MAX_WAYPOINTS: usize = 12;
pub const MAX_NAME_LENGTH: usize = 15;

#[derive(Clone)]
pub struct Waypoint {
    pub name: String<MAX_NAME_LENGTH>,
    pub coordinate: Coordinate,
}


pub type WaypointList = heapless::Vec<Waypoint, MAX_WAYPOINTS>;
static WAYPOINTS: Mutex<CriticalSectionRawMutex, WaypointList> = Mutex::new(Vec::new());

pub async fn get_num_waypoints() -> usize {
    let ways = WAYPOINTS.lock().await;
    return ways.len();
}

pub async fn get_waypoint_at_index(i: usize) -> Option<Waypoint> {
    let ways = WAYPOINTS.lock().await;
    ways.get(i).cloned()
}

pub async fn get_all_waypoints() -> WaypointList {
    WAYPOINTS.lock().await.clone()
}

pub async fn replace_waypoints(waypoints: WaypointList) {
    let mut ways = WAYPOINTS.lock().await;
    *ways = waypoints;
}
