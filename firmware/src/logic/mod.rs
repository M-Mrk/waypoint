use embassy_sync::blocking_mutex::{CriticalSectionMutex, Mutex, raw::CriticalSectionRawMutex};
use esp_storage::FlashStorage;
use heapless::Vec;

use crate::navigation::Waypoint;

pub type WaypointList = heapless::Vec<Waypoint, 12>;

static waypoints: Mutex<CriticalSectionRawMutex, WaypointList> = Mutex::new(Vec::new());

#[embassy_executor::task]
pub async fn logic_task() {
    // let mut perm_storage = FlashStorage::new(flash);


    loop {
    }
}
