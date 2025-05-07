use std::fmt;

use chrono::{Datelike, Duration, FixedOffset, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use redis::{Client as RDBClient, Commands, Connection as RDBConn};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct RedisTravelHistory {
    #[serde(rename = "from")]
    pub from: String,
    #[serde(rename = "ride_distance")]
    pub ride_distance: f32,
    #[serde(rename = "start_dttm")]
    pub start_dttm: u64,
    #[serde(rename = "stop_dttm")]
    pub stop_dttm: u64,
    #[serde(rename = "to")]
    pub to: String,
    #[serde(rename = "top_speed")]
    pub top_speed: f32,
    #[serde(rename = "energy_consumed")]
    pub energy_consumed: f64,
    #[serde(rename = "avg_speed")]
    pub avg_speed: Option<f32>,
    #[serde(rename = "incognito")]
    pub incognito: Option<bool>,
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
enum BikeRideModes {
    #[default]
    glide = 4,
    combat = 2,
    ballistic = 1,
}

impl fmt::Display for BikeRideModes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

pub fn get_rdb_conn() -> RDBConn {
    println!("Entering the get_rdb_conn func: ",);
    let client = RDBClient::open(
        "redis://prod-redis.64wnxk.clustercfg.memorydb.ap-south-1.amazonaws.com:6379",
    )
    .unwrap();
    let conn = client.get_connection_with_timeout(std::time::Duration::new(5, 0));
    let connection = match conn {
        Ok(_) => conn.unwrap(),
        Err(e) => {
            println!("redis server connection problem: {:?}", e);
            panic!("redis server connection problem {:?}", e);
        }
    };
    println!("redis server connected");
    println!("Exiting the get_rdb_conn");
    return connection;
}

// pub fn get_timestamp_for_day(day: i64) -> u64 {
//     let utcdate: NaiveDateTime = Utc::now()
//         .date_naive()
//         .and_hms_opt(0, 0, 0)
//         .unwrap_or_default();
//     let date: NaiveDateTime = utcdate
//         .checked_add_signed(Duration::days(day))
//         .unwrap_or_default();
//     let timestamp = date.timestamp();
//     return timestamp as u64;
// }

pub fn get_ist_timestamp_for_day(day: i64) -> u64 {
    println!("Entering  get_ist_timestamp_for_day");
    let no_of_seconds = 60i32;
    let no_of_minutes = 60i32;

    let utcdate = Utc::now().date_naive();
    let datetime =
        FixedOffset::east_opt(5i32 * no_of_minutes * no_of_seconds + 30i32 * no_of_seconds)
            .unwrap()
            .with_ymd_and_hms(utcdate.year(), utcdate.month(), utcdate.day(), 0, 0, 0)
            .unwrap();

    let date = datetime
        .checked_add_signed(Duration::days(day))
        .unwrap_or_default();
    let timestamp = date.timestamp();
    println!(
        "Exiting  get_ist_timestamp_for_day with timestamp: {:?}",
        timestamp
    );
    return timestamp as u64;
}

pub async fn get_range_all_modes(bike_id: &String) -> (String, String, String) {
    println!("Entering get_range_all_modes");
    let conn = &mut get_rdb_conn();
    let glide_range: String = conn
        .get(&format!("{{{}}}_range_glide", bike_id))
        .unwrap_or("".to_string());

    let combat_range: String = conn
        .get(&format!("{{{}}}_range_combat", bike_id))
        .unwrap_or("".to_string());

    let ballistic_range: String = conn
        .get(&format!("{{{}}}_range_ballistic", bike_id))
        .unwrap_or("".to_string());
    println!(
        "Exiting get_range_all_modes: {:?}, {:?}, {:?}",
        glide_range, combat_range, ballistic_range
    );
    return (glide_range, combat_range, ballistic_range);
}

pub async fn get_trip_hist_day(
    conn: &mut RDBConn,
    bike_id: &String,
    day: Option<i64>,
) -> Vec<RedisTravelHistory> {
    println!("Entering get_trip_hist_day");
    let mut travel_history_vec: Vec<RedisTravelHistory> = Vec::<RedisTravelHistory>::new();

    let day_tstmp: u64 = get_ist_timestamp_for_day(day.unwrap_or(0i64));
    println!("day_tstmp #### {:?}", day_tstmp);
    let mem_db_key: String = format!("{{{}}}_{}_trips", bike_id, day_tstmp);
    let response: Vec<String> = conn.lrange(mem_db_key, 0, -1).unwrap();

    for item in response {
        let travel_history_item_str: String = conn.get(item).unwrap_or("".to_string());
        if travel_history_item_str != "".to_string() {
            let mut trip_hist_item =
                serde_json::from_str::<RedisTravelHistory>(&travel_history_item_str).unwrap();
            let start_millis = chrono::NaiveDateTime::from_timestamp_opt(
                trip_hist_item.start_dttm.try_into().unwrap(),
                0,
            )
            .unwrap();
            let stop_millis = chrono::NaiveDateTime::from_timestamp_opt(
                trip_hist_item.stop_dttm.try_into().unwrap(),
                0,
            )
            .unwrap();
            let diff_sec = (stop_millis - start_millis).num_seconds() as f32;
            let trip_avg_speed = (trip_hist_item.ride_distance * 3600.00f32) / diff_sec;
            trip_hist_item.avg_speed = Some(trip_avg_speed);
            travel_history_vec.push(trip_hist_item);
        }
    }
    println!(
        "Exiting get_trip_hist_day travel_history_vec: {:?}",
        travel_history_vec
    );
    return travel_history_vec;
}

pub async fn get_trip_hist_lastweek(
    conn: &mut RDBConn,
    bike_id: &String,
) -> Vec<RedisTravelHistory> {
    println!("Entering the get_trip_hist_lastweek");
    let mut travel_history_vec: Vec<RedisTravelHistory> = Vec::<RedisTravelHistory>::new();

    for day in -6..=0 {
        let day_tstmp: u64 = get_ist_timestamp_for_day(day);
        println!("day_tstmp #### {:?}", day_tstmp);
        let mem_db_key: String = format!("{{{}}}_{}_trips", bike_id, day_tstmp);
        let response: Vec<String> = conn.lrange(mem_db_key, 0, -1).unwrap();

        for item in response {
            let travel_history_item_str: String = conn.get(item).unwrap_or("".to_string());
            if travel_history_item_str != "".to_string() {
                let mut trip_hist_item =
                    serde_json::from_str::<RedisTravelHistory>(&travel_history_item_str).unwrap();
                let start_millis = chrono::NaiveDateTime::from_timestamp_opt(
                    trip_hist_item.start_dttm.try_into().unwrap(),
                    0,
                )
                .unwrap();
                let stop_millis = chrono::NaiveDateTime::from_timestamp_opt(
                    trip_hist_item.stop_dttm.try_into().unwrap(),
                    0,
                )
                .unwrap();
                let diff_sec = (stop_millis - start_millis).num_seconds() as f32;
                let trip_avg_speed = (trip_hist_item.ride_distance * 3600.00f32) / diff_sec;
                trip_hist_item.avg_speed = Some(trip_avg_speed);
                travel_history_vec.push(trip_hist_item);
            }
        }
    }
    println!(
        "Exiting the get_trip_hist_lastweek with travel_history_vec: {:?}",
        travel_history_vec
    );
    return travel_history_vec;
}

pub async fn get_distance_lastweek(bike_id: &String) -> u16 {
    println!("Entering get_distance_lastweek");
    let lw_total_distance: u16;

    let lw_trip_hist = get_trip_hist_lastweek(&mut get_rdb_conn(), bike_id).await;
    let mut total_dist: f32 = 0f32;
    for trip_hist in lw_trip_hist {
        total_dist += trip_hist.ride_distance as f32
    }
    lw_total_distance = total_dist.round() as u16;
    println!(
        "Exiting the get_distance_lastweek: lw_total_distance: {:?}",
        lw_total_distance
    );
    return lw_total_distance;
}

pub async fn get_vcu_data(
    bike_id: &String,
) -> (String, String, u32, u32, u32, String, u64, String) {
    println!("Entering the get_vcu_data");
    let conn = &mut get_rdb_conn();

    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap();
    let in_s = (since_the_epoch.as_millis() / 1000) as u64;

    let bike_status: String = conn
        .get(&format!("{{{}}}_bike_status", bike_id))
        .unwrap_or("".to_string());

    let soc: u32 = conn
        .get(&format!("{{{}}}_soc", bike_id))
        .unwrap_or("0.0".to_string())
        .parse::<f64>()
        .unwrap()
        .round() as u32;

    let full_charge_eta: u32 = conn
        .get(&format!("{{{}}}_full_charge_eta", bike_id))
        .unwrap_or("0".to_string())
        .parse::<u64>()
        .unwrap() as u32;

    let ride_mode = conn
        .get(&format!("{{{}}}_current_mode", bike_id))
        .unwrap_or("glide".to_string());

    let range_left = conn
        .get(&format!("{{{}}}_range_{}", bike_id, ride_mode))
        .unwrap_or("270".to_string());
    let odometer = conn
        .get(&format!("{{{}}}_odo", bike_id))
        .unwrap_or("0".to_string())
        .parse::<f64>()
        .unwrap()
        .round() as u32;

    let session_id = conn
        .get(&format!("{{{}}}_sessionid", bike_id))
        .unwrap_or("0".to_string())
        .parse::<u64>()
        .unwrap();

    let mut last_timestamp = conn
        .get(&format!("{{{}}}_last_timestamp", bike_id))
        .unwrap_or(
            conn.get(&format!("{{{}}}_soc_timestamp", bike_id))
                .unwrap_or(in_s.to_string())
                .to_string(),
        )
        .parse::<u64>()
        .unwrap();

    let charger_type = conn
        .get(&format!("{{{}}}_charger_type", bike_id))
        .unwrap_or("202".to_string());

    if last_timestamp == 0u64 {
        last_timestamp = in_s;
    }

    println!(
        "Exiting the get_vcu_data: {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}",
        ride_mode,
        range_left,
        soc,
        odometer,
        full_charge_eta,
        bike_status,
        last_timestamp,
        charger_type
    );

    return (
        ride_mode.to_string(),
        range_left.to_string(),
        soc,
        odometer,
        full_charge_eta,
        bike_status,
        last_timestamp,
        charger_type,
    );
}

pub async fn get_last_known_location(bike_id: &String) -> (f64, f64) {
    println!("Entering get_last_known_location");
    let conn = &mut get_rdb_conn();
    let gloc_data: String = conn
        .get(format!("{{{}}}_lloc", bike_id.to_string()))
        .unwrap_or("".to_string());

    if gloc_data != "" {
        let gloc_data_json: Value = serde_json::from_str(&gloc_data).unwrap();

        let lat = gloc_data_json.get("lat").unwrap().as_f64().unwrap();

        let lng = gloc_data_json.get("lng").unwrap().as_f64().unwrap();

        println!("Exiting the get_last_known_location: {:?}, {:?}", lat, lng);

        return (lat, lng);
    } else {
        return (0.0f64, 0.0f64);
    }
}

pub async fn get_trip_hist_last(conn: &mut RDBConn, bike_id: &String) -> RedisTravelHistory {
    println!("Entering the get_last_trip_hist_last");
    let mut last_travel_history: RedisTravelHistory = RedisTravelHistory::default();

    let mut got_last_travel_history = false;
    let mut day: i64 = 0;
    while !got_last_travel_history && day > -30 {
        let day_tstmp: u64 = get_ist_timestamp_for_day(day);
        let mem_db_key: String = format!("{{{}}}_{}_trips", bike_id, day_tstmp);
        let response: Vec<String> = conn.lrange(mem_db_key, 0, -1).unwrap();

        if response.len() != 0 {
            got_last_travel_history = true;

            for item in response {
                let travel_history_item_str: String = conn.get(item).unwrap_or("".to_string());

                if travel_history_item_str != "".to_string() {
                    let mut trip_hist_item =
                        serde_json::from_str::<RedisTravelHistory>(&travel_history_item_str)
                            .unwrap();
                    let start_millis = chrono::NaiveDateTime::from_timestamp_opt(
                        trip_hist_item.start_dttm.try_into().unwrap(),
                        0,
                    )
                    .unwrap();
                    let stop_millis = chrono::NaiveDateTime::from_timestamp_opt(
                        trip_hist_item.stop_dttm.try_into().unwrap(),
                        0,
                    )
                    .unwrap();
                    let diff_sec = (stop_millis - start_millis).num_seconds() as f32;
                    let trip_avg_speed = (trip_hist_item.ride_distance * 3600.00f32) / diff_sec;
                    trip_hist_item.avg_speed = Some(trip_avg_speed);
                    last_travel_history = trip_hist_item;
                }
            }
        } else {
            day -= 1;
        }
    }
    println!(
        "Exitring the get_trip_hist_last last_travel_history: {:?}",
        last_travel_history
    );
    return last_travel_history;
}
